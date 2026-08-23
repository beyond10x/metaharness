import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';
import Heading from '@theme/Heading';

import styles from './index.module.css';

const PROMISES = [
  {
    title: 'Unified',
    body: 'One event stream and one command set. Everything harness-specific lives in that harness’s adapter crate and nowhere else.',
    to: '/docs/protocol/events',
    cta: 'The wire',
  },
  {
    title: 'Hermetic',
    body: 'A run shares credentials with the operator and nothing else. Twelve rows, each imposed and each asserted from the run’s own record — never from the config that was supposed to produce it.',
    to: '/docs/hermetic',
    cta: 'The contract',
  },
  {
    title: 'In control at every step',
    body: 'Which tools the harness may call is decided per call, by the embedder, through the protocol — not once at launch.',
    to: '/docs/control-seam',
    cta: 'The seam',
  },
];

const SHAPE = `              events out (JSONL) ─────────▶  your process / your workflow engine
metaharness
              commands in (steering) ◀─────  approve / deny a tool call, inject, halt`;

function Hero() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={styles.hero}>
      <div className="container">
        <Heading as="h1" className={styles.heroTitle}>
          {siteConfig.title}
        </Heading>
        <p className={styles.heroTagline}>{siteConfig.tagline}</p>
        <p className={styles.heroBody}>
          A harness — Claude Code, Codex, the next one — keeps its own loop, its own tools and its
          own credentials. metaharness drives it <strong>from outside</strong>, and makes the run
          the same shape regardless of which harness is inside.
        </p>
        <div className={styles.heroButtons}>
          <Link className="button button--primary button--lg" to="/docs/quickstart">
            Quickstart
          </Link>
          <Link className="button button--secondary button--lg" to="/docs/">
            What it is
          </Link>
        </div>
      </div>
    </header>
  );
}

function Shape() {
  return (
    <section className={styles.section}>
      <div className={clsx('container', styles.narrow)}>
        <CodeBlock language="text">{SHAPE}</CodeBlock>
      </div>
    </section>
  );
}

function Promises() {
  return (
    <section className={styles.section}>
      <div className="container">
        <div className="row">
          {PROMISES.map((promise) => (
            <div key={promise.title} className={clsx('col col--4', styles.promiseCol)}>
              <div className={styles.promise}>
                <Heading as="h3">{promise.title}</Heading>
                <p>{promise.body}</p>
                <Link to={promise.to}>{promise.cta} →</Link>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function Claim() {
  return (
    <section className={clsx(styles.section, styles.claim)}>
      <div className={clsx('container', styles.narrow)}>
        <Heading as="h2">The claim this exists to be able to make</Heading>
        <p>
          A frame that admitted no shell was given a prompt that asked for one. metaharness denied
          the call at the hook. The call did not run. And{' '}
          <strong>the vendor’s own record said so</strong> — on both vendors.
        </p>
        <p className={styles.claimFooter}>
          That is the one thing no free test tier can reach. <Link to="/docs/status">Status →</Link>
        </p>
      </div>
    </section>
  );
}

export default function Home() {
  return (
    <Layout
      title="One interface to many agent harnesses"
      description="Drive Claude Code, Codex and the next harness through one event stream and one command set — observable, steerable and hermetic.">
      <Hero />
      <main>
        <Shape />
        <Promises />
        <Claim />
      </main>
    </Layout>
  );
}
