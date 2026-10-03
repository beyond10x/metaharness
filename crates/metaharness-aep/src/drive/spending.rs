//! Explicit model-spend authority, independent of workflow and retry limits.
use super::{Launch, ReserveError, SpendBudget, SpendTerms, StepContext, StepMap};
use anyhow::{Context, Result, bail};
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, clap::Args)]
pub struct SpendOptions {
    /// Explicitly remove only the outer USD cap. Workflow and retry limits still apply.
    #[arg(long, requires = "spend_authorization", conflicts_with = "budget_usd")]
    pub uncapped_budget: bool,
    /// Operator authorization reference retained in launch policy and invocation ledger.
    #[arg(long, requires = "uncapped_budget", value_name = "REFERENCE")]
    pub spend_authorization: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum SpendPolicy {
    Finite(SpendTerms),
    Uncapped { authorization_ref: String },
}

fn authority(reference: &str) -> Result<()> {
    if reference.trim().is_empty() {
        bail!("uncapped spending requires a nonempty --spend-authorization reference");
    }
    Ok(())
}

pub(super) fn policy(
    map: &StepMap,
    previous: Option<&Launch>,
    budget: Option<&str>,
    charge: Option<&str>,
    options: &SpendOptions,
    live: bool,
) -> Result<Option<SpendPolicy>> {
    if !super::has_llm_steps(map) {
        return Ok(None);
    }
    if !live {
        bail!("METAHARNESS_LIVE=1 is required before a model invocation may be admitted");
    }
    if options.uncapped_budget != options.spend_authorization.is_some() {
        bail!("explicit --uncapped-budget and --spend-authorization must be supplied together");
    }
    if let Some(reference) = &options.spend_authorization {
        authority(reference)?;
    }
    let remembered = previous
        .map(|launch| {
            if let Some(value) = launch.extra.get("spend_policy") {
                serde_json::from_value::<Option<SpendPolicy>>(value.clone())
                    .context("invalid remembered spend policy")
            } else {
                launch
                    .extra
                    .get("spend")
                    .map(|value| serde_json::from_value::<Option<SpendTerms>>(value.clone()))
                    .transpose()
                    .context("invalid legacy spend terms")
                    .map(|terms| terms.flatten().map(SpendPolicy::Finite))
            }
        })
        .transpose()?;
    if let Some(remembered) = remembered {
        return resume_policy(map, remembered, budget, charge, options);
    }
    if options.uncapped_budget {
        if budget.is_some() || charge.is_some() {
            bail!("uncapped spending cannot carry a finite cap or assumed charge");
        }
        let reference = options
            .spend_authorization
            .as_deref()
            .context("uncapped spending requires --spend-authorization")?;
        authority(reference)?;
        return Ok(Some(SpendPolicy::Uncapped {
            authorization_ref: reference.to_owned(),
        }));
    }
    if options.spend_authorization.is_some() {
        bail!("--spend-authorization requires explicit --uncapped-budget");
    }
    super::spend_terms_with_live(map, budget, charge, live)
        .map(|terms| terms.map(SpendPolicy::Finite))
}

fn resume_policy(
    map: &StepMap,
    remembered: Option<SpendPolicy>,
    budget: Option<&str>,
    charge: Option<&str>,
    options: &SpendOptions,
) -> Result<Option<SpendPolicy>> {
    match remembered {
        Some(SpendPolicy::Uncapped { authorization_ref }) => {
            authority(&authorization_ref)?;
            if budget.is_some() || charge.is_some() {
                bail!("resume cannot change an uncapped policy to finite terms");
            }
            if options
                .spend_authorization
                .as_ref()
                .is_some_and(|value| value != &authorization_ref)
            {
                bail!("resume cannot change the remembered spend authorization");
            }
            Ok(Some(SpendPolicy::Uncapped { authorization_ref }))
        }
        finite => {
            if options.uncapped_budget || options.spend_authorization.is_some() {
                bail!("resume cannot widen a finite or missing spend policy to uncapped");
            }
            let terms = finite.and_then(|policy| match policy {
                SpendPolicy::Finite(terms) => Some(terms),
                SpendPolicy::Uncapped { .. } => None,
            });
            super::resumed_spend_terms_with_live(map, terms, budget, true)
                .map(|terms| terms.map(SpendPolicy::Finite))
        }
    }
}

#[derive(Debug)]
pub(super) enum AdmissionBudget {
    Finite(SpendBudget),
    Uncapped(UncappedBudget),
}

impl AdmissionBudget {
    pub(super) fn open(directory: &Path, policy: SpendPolicy, resumed: bool) -> Result<Self> {
        match policy {
            SpendPolicy::Finite(terms) => Ok(Self::Finite(if resumed {
                SpendBudget::resume(directory, terms)?
            } else {
                SpendBudget::start(directory, terms)?
            })),
            SpendPolicy::Uncapped { authorization_ref } => Ok(Self::Uncapped(
                UncappedBudget::open(directory, authorization_ref, resumed)?,
            )),
        }
    }

    pub(super) fn reserve(
        &mut self,
        context: &StepContext<'_>,
    ) -> std::result::Result<(), ReserveError> {
        match self {
            Self::Finite(budget) => budget.reserve(),
            Self::Uncapped(budget) => budget
                .reserve(context)
                .map_err(|error| ReserveError::Persist(error.to_string())),
        }
    }

    pub(super) fn observe(&mut self, transcript: &Path) -> Result<()> {
        match self {
            Self::Finite(_) => Ok(()),
            Self::Uncapped(budget) => budget.observe(transcript),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Invocation {
    ordinal: u64,
    task: String,
    state: String,
    step: usize,
    attempt: u32,
    observed_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct UncappedLedger {
    format: String,
    policy: SpendPolicy,
    invocations: Vec<Invocation>,
}

#[derive(Debug)]
pub(super) struct UncappedBudget {
    path: PathBuf,
    ledger: UncappedLedger,
}

impl UncappedBudget {
    fn open(directory: &Path, authorization_ref: String, resumed: bool) -> Result<Self> {
        authority(&authorization_ref)?;
        let path = directory.join(super::SPEND_FILE);
        let policy = SpendPolicy::Uncapped { authorization_ref };
        let ledger = if resumed {
            let ledger: UncappedLedger = serde_json::from_slice(&fs::read(&path)?)
                .context("invalid uncapped invocation ledger")?;
            if ledger.format != "metaharness.drive-spend/2" || ledger.policy != policy {
                bail!("uncapped ledger format or authorization differs from the launch policy");
            }
            for (index, invocation) in ledger.invocations.iter().enumerate() {
                if invocation.ordinal
                    != u64::try_from(index)?
                        .checked_add(1)
                        .context("invocation count overflow")?
                    || invocation.task.is_empty()
                    || invocation.state.is_empty()
                    || invocation.attempt == 0
                    || invocation
                        .observed_cost_usd
                        .is_some_and(|cost| !cost.is_finite() || cost < 0.0)
                {
                    bail!(
                        "uncapped invocation ledger has inconsistent identity, ordering or observed cost"
                    );
                }
            }
            ledger
        } else {
            if path.exists() {
                bail!("a spend ledger already exists; start cannot overwrite invocation authority");
            }
            UncappedLedger {
                format: "metaharness.drive-spend/2".to_owned(),
                policy,
                invocations: Vec::new(),
            }
        };
        let budget = Self { path, ledger };
        if !resumed {
            budget.persist()?;
        }
        Ok(budget)
    }

    fn reserve(&mut self, context: &StepContext<'_>) -> Result<()> {
        let ordinal = u64::try_from(self.ledger.invocations.len())?
            .checked_add(1)
            .context("invocation count overflow")?;
        self.ledger.invocations.push(Invocation {
            ordinal,
            task: context.task.id.to_string(),
            state: context.state.to_string(),
            step: context.index,
            attempt: context.attempt,
            observed_cost_usd: None,
        });
        // Failure grants no spawn. Keep the in-memory admission conservatively consumed.
        self.persist()
    }

    fn observe(&mut self, transcript: &Path) -> Result<()> {
        let mut cost = None;
        for line in std::io::BufReader::new(fs::File::open(transcript)?).lines() {
            let event: serde_json::Value = serde_json::from_str(&line?)?;
            if event["event"] == "session.ended" {
                cost = event["total_cost_usd"]
                    .as_f64()
                    .filter(|value| value.is_finite() && *value >= 0.0);
            }
        }
        self.ledger
            .invocations
            .last_mut()
            .context("no invocation admitted before observation")?
            .observed_cost_usd = cost;
        self.persist()
    }

    fn persist(&self) -> Result<()> {
        let next = self.path.with_extension("json.next");
        let mut file = fs::File::create(&next).context("creating uncapped ledger replacement")?;
        serde_json::to_writer_pretty(&mut file, &self.ledger)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&next, &self.path).context("publishing uncapped invocation admission")?;
        fs::File::open(self.path.parent().context("ledger has no parent")?)?.sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "spending_tests.rs"]
mod tests;
