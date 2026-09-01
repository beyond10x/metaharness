# Sandbox inversion — design v0.1

Status: rejected for production implementation by the operator on 2026-09-01. The scripted,
harness-neutral contract released in `0.4.0` remains as historical evidence vocabulary; it does
not authorize a Substrate adapter, an outer vendor sandbox, or adapter migration. The proposal is
retained below so the rejected boundary is not rediscovered as unfinished work.

## 1. Outcome

A future vendor adapter asks for one harness-neutral process envelope. The vendor process can see
only the declared workspace, scratch state, staged executables and credential channel; it can reach
only the model proxy. Vendor sandbox flags are disabled inside that envelope where the vendor
offers a documented switch, so one outer policy—not a stack of differently ordered vendor
policies—decides the reachable machine.

The inversion is complete only when the run record proves the resulting process view. Passing a
flag or constructing a mount list is not evidence that the child was confined.

## 2. Boundary decision

Metaharness owns the historical envelope attestation vocabulary. It does not implement Linux
namespaces, Landlock, containers or a filesystem sandbox in this repository: sandboxed execution
belongs to Substrate, and Metaharness owns no independent Substrate pin (Atlas ADR 0019).

The mechanism enters through an injected `ProcessEnvelope` port supplied by the embedder. The port
takes a sealed value and returns a child process plus measured facts. Tests use a scripted port;
there is no production provider. No adapter imports the mechanism and no adapter may widen the
sealed request.

This rejects both tempting shortcuts:

- spawning `bwrap` directly from each adapter would make every adapter an execution sandbox and
  duplicate failure handling;
- accepting a prebuilt command prefix would be untyped authority: metaharness could neither prove
  what it mounted nor compare the result with the request.

## 3. Sealed envelope request

The request is a value with no ambient fallbacks:

- canonical read-only runtime roots required by the resolved executable;
- one workspace root and an ordered set of writable subtrees;
- one scratch state root, private to the run;
- staged executable files, each with digest and mounted path;
- a constructed environment, including the scratch config roots;
- a credential channel reference, never the credential bytes;
- network `none` or `model_proxy`, where the latter identifies one loopback or Unix-socket proxy;
- process, wall-time and output bounds.

The value is sealed before the port receives it. A digest of the canonical request enters
`session.started`; adapters receive only resolved child handles, never a mutable mount list.

## 4. Measured result and hermetic rows

The port returns facts observed from the child boundary: mount table, writable paths, environment
key set, executable digests, network namespace/proxy reachability, process limits and the actual
cwd. Metaharness compares request and result and then emits imposed controls.

- H2: config roots are inside scratch and the measured mount contains them there.
- H3: the measured environment keys equal the constructed key set.
- H7: the measured cwd is the declared workspace and no additional writable root exists.
- H8: only explicitly staged hook programs exist in the executable set.
- H11: no operator home or ancestor project memory is mounted.
- Network isolation becomes a new versioned row only when `none` and `model_proxy` have both been
  negatively tested: an undeclared destination must fail from inside the envelope.

Silence from the port is `unk`, never imposed. A request/result mismatch is a launch refusal under
strict hermeticity and a named gap otherwise.

## 5. Credential and proxy shape

Credential custody stays outside the sandbox. The loopback provider proxy holds the live token and
the child receives only a per-run placeholder over the declared channel. For a network-isolated
child, the proxy endpoint must be part of the envelope itself: either a Unix socket mounted into
the namespace or a proxy process launched inside it. Host loopback is not assumed to be the
child's loopback.

No credential file is mounted. No secret appears in the envelope request, digest, environment
attestation, error or transcript.

## 6. Vendor adapters under inversion

Each adapter declares three independently verified facts:

1. how to disable or minimize its own sandbox and permission prompts;
2. which runtime files and executables it needs read-only;
3. which record proves the vendor did not silently re-enable an inner policy.

An adapter with no verified disable switch may still run inside the outer envelope, but the run
records nested enforcement and cannot claim the treatment is constant across vendors. An adapter
with no per-call seam does not gain one from the envelope: process confinement answers reach, not
which admitted call the embedder approves.

## 7. Rollout and acceptance

1. Add the protocol values and a scripted `ProcessEnvelope` port; prove digest, mismatch refusal
   and absence-as-unknown without spawning a process.
2. ~~Implement a production provider and drive negative mount, write and network probes.~~ Rejected.
3. ~~Migrate one vendor adapter behind an opt-in flag.~~ Rejected.
4. ~~Migrate the second vendor and remove the opt-in.~~ Rejected.

The released scripted contract continues to prove digest stability, request/result mismatch and
absence-as-unknown. It makes no claim that a real vendor process was placed inside that envelope.
