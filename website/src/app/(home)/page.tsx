import Link from 'next/link';
import { Activity, ArrowRight, BookOpen, Sigma, Workflow } from 'lucide-react';
import { Tab, Tabs } from 'fumadocs-ui/components/tabs';
import { Card, Cards } from '@/components/card';
import { Highlight } from '@/lib/highlight';

const INSTALL: { name: string; lang: string; code: string }[] = [
  { name: 'Python', lang: 'bash', code: 'pip install sentil' },
  { name: 'Rust', lang: 'bash', code: 'cargo add sentil' },
  { name: 'C', lang: 'bash', code: 'vcpkg install sentil' },
  { name: 'C++', lang: 'bash', code: 'vcpkg install sentil-cpp' },
  {
    name: 'Java',
    lang: 'xml',
    code: `<dependency>
  <groupId>io.github.sedislab</groupId>
  <artifactId>sentil</artifactId>
  <version>0.3.0</version>
</dependency>`,
  },
  { name: 'Julia', lang: 'julia', code: '] add Sentil' },
  {
    name: 'MATLAB',
    lang: 'matlab',
    code: `% after downloading Sentil.mltbx from the File Exchange or a GitHub release
matlab.addons.toolbox.installToolbox('Sentil.mltbx')`,
  },
  {
    name: 'CLI',
    lang: 'bash',
    code: `# macOS or Linux
brew install sedislab/sentil/sentil

# Windows
winget install SEDIS.SENTIL`,
  },
  {
    name: 'ROS 2',
    lang: 'bash',
    code: `# swap humble for jazzy or rolling
sudo apt install ros-humble-sentil-ros`,
  },
];

const MONITOR: { name: string; lang: string; code: string; out?: string }[] = [
  {
    name: 'Python',
    lang: 'python',
    code: `import sentil
from sentil import Formula

trace = sentil.Trace([0, 1, 2, 3, 4], {"speed": [12, 9, 7, 4, 6]})
phi = Formula.parse("G (speed > 5)")

print(phi.robustness(trace))  # -1.0`,
  },
  {
    name: 'Rust',
    lang: 'rust',
    code: `use sentil::{Formula, Trace};

fn main() -> sentil::Result<()> {
    let mut trace = Trace::new(vec![0.0, 1.0, 2.0, 3.0, 4.0])?;
    trace.add_signal("speed", vec![12.0, 9.0, 7.0, 4.0, 6.0])?;

    let phi = Formula::parse("G (speed > 5)")?;
    println!("robustness: {}", phi.robustness(&trace)?); // robustness: -1
    Ok(())
}`,
  },
  {
    name: 'C',
    lang: 'c',
    code: `#include <stdio.h>
#include <sentil.h>

int main(void) {
    const double times[] = {0, 1, 2, 3, 4};
    const double speed[] = {12, 9, 7, 4, 6};
    sentil_trace_t *trace = sentil_trace_from_signal(times, 5, "speed", speed, 5);
    sentil_formula_t *phi = sentil_formula_parse("G (speed > 5)");
    double rho;
    if (!trace || !phi || sentil_formula_robustness(phi, trace, &rho) != SENTIL_OK) {
        fprintf(stderr, "%s\\n", sentil_get_last_error());
        return 1;
    }
    printf("%f\\n", rho);
    sentil_formula_destroy(phi);
    sentil_trace_destroy(trace);
    return 0;
}`,
    out: '-1.000000',
  },
  {
    name: 'C++',
    lang: 'cpp',
    code: `#include <sentil/sentil.hpp>
#include <iostream>

int main() {
    sentil::Trace trace({0, 1, 2, 3, 4}, "speed", {12.0, 9.0, 7.0, 4.0, 6.0});
    sentil::Formula phi = sentil::Formula::parse("G (speed > 5)");

    std::cout << "robustness: " << phi.robustness(trace) << "\\n";
    for (const sentil::Interval& v : phi.violations(trace)) {
        std::cout << "violation [" << v.start << ", " << v.end << "]\\n";
    }
    return 0;
}`,
    out: `robustness: -1
violation [0, 3]`,
  },
  {
    name: 'Java',
    lang: 'java',
    code: `import io.github.sedislab.sentil.Formula;
import io.github.sedislab.sentil.Trace;

try (Trace trace = Trace.create(new double[] {0, 1, 2, 3, 4});
        Formula phi = Formula.parse("G (speed > 5)")) {
    trace.addSignal("speed", new double[] {12, 9, 7, 4, 6});
    System.out.println(phi.robustness(trace));   // -1.0
}`,
  },
  {
    name: 'Julia',
    lang: 'julia',
    code: `using Sentil

phi = formula("G (speed > 5)")
trace = Trace(collect(0.0:1.0:4.0), "speed", [12.0, 9.0, 7.0, 4.0, 6.0])
robustness(phi, trace)   # -1.0`,
  },
  {
    name: 'MATLAB',
    lang: 'matlab',
    code: `trace = sentil.Trace([0 1 2 3 4], 'speed', [12 9 7 4 6]);
phi = sentil.Formula.parse('G (speed > 5)');

fprintf('robustness: %g\\n', phi.robustness(trace));`,
    out: 'robustness: -1',
  },
  {
    name: 'CLI',
    lang: 'bash',
    code: "sentil check -f 'G (speed > 5)' -t speeds.csv",
    out: `check
  formula     G (speed > 5)
  trace       speeds.csv
  semantics   dense
  verdict     violated
  robustness  -1.000000`,
  },
];

export default function HomePage() {
  return (
    <div id="main-content" tabIndex={-1} className="mx-auto w-full max-w-3xl px-6 pt-14 pb-20 outline-none">
      <h1 className="font-display text-3xl font-bold tracking-tight">SENTIL</h1>
      <p className="mt-4 text-lg leading-8 text-fd-muted-foreground">
        A tool for Signal Temporal Logic and its Probabilistic extension, PrSTL. It monitors a signal in real time, checks a probabilistic specification, and synthesizes inputs and controllers that satisfy it.
      </p>
      <div className="mt-5 flex flex-wrap gap-x-6 gap-y-2 text-sm font-medium">
        <Link href="/docs/start" prefetch={false} className="inline-flex items-center gap-1 text-fd-primary">
          Get started
          <ArrowRight className="size-3.5" />
        </Link>
        <Link href="/playground" prefetch={false} className="inline-flex items-center gap-1 text-fd-primary">
          Open the playground
          <ArrowRight className="size-3.5" />
        </Link>
        <Link href="/docs/start/tutorial" prefetch={false} className="inline-flex items-center gap-1 text-fd-primary">
          Read the tutorial
          <ArrowRight className="size-3.5" />
        </Link>
      </div>

      <h2 className="mt-12 font-display text-xl font-bold tracking-tight">Install</h2>
      <Tabs groupId="lang" persist items={INSTALL.map((i) => i.name)}>
        {INSTALL.map((i) => (
          <Tab key={i.name} value={i.name}>
            <div className="landing-frame">
              <div className="landing-code">
                <Highlight code={i.code} lang={i.lang} />
              </div>
            </div>
          </Tab>
        ))}
      </Tabs>
      <p className="mt-5 leading-7 text-fd-muted-foreground">
        Every package carries the compiled engine, so nothing above needs building. The{' '}
        <Link href="/docs/start/install" prefetch={false} className="text-fd-primary">
          install page
        </Link>{' '}
        adds the prerequisites, a check that each path landed, and{' '}
        <Link href="/docs/start/install#from-source" prefetch={false} className="text-fd-primary">
          the build from source
        </Link>
        . One page per language lives in the{' '}
        <Link href="/docs/languages" prefetch={false} className="text-fd-primary">
          languages section
        </Link>
        , from install through the complete API.
      </p>
      <p className="mt-3 leading-7 text-fd-muted-foreground">
        <Link href="/docs/languages/embedded" prefetch={false} className="text-fd-primary">
          Microcontrollers
        </Link>{' '}
        take a library through Arduino, PlatformIO, ESP-IDF, Zephyr, or a bare-metal archive. The{' '}
        <Link href="/docs/languages/apollo" prefetch={false} className="text-fd-primary">
          Apollo
        </Link>{' '}
        module is source you drop into a workspace and build with Bazel, and the{' '}
        <Link href="/docs/languages/autosar" prefetch={false} className="text-fd-primary">
          AUTOSAR Adaptive
        </Link>{' '}
        applications ship as packages that run against a stub or build against a vendor stack.
      </p>

      <h2 className="mt-12 font-display text-xl font-bold tracking-tight">The first monitor</h2>
      <p className="mt-2 leading-7 text-fd-muted-foreground">
        We monitor five samples of a speed signal against a specification. The speed dips to 4 at t=3 but the rule ask that speed be greater than 5, so the robustness is -1.
      </p>
      <Tabs groupId="lang" persist items={MONITOR.map((m) => m.name)}>
        {MONITOR.map((m) => (
          <Tab key={m.name} value={m.name}>
            <div className="landing-frame">
              <div className="landing-code">
                <Highlight code={m.code} lang={m.lang} />
              </div>
              {m.out && <div className="landing-out">{m.out}</div>}
            </div>
          </Tab>
        ))}
      </Tabs>

      <h2 className="mt-12 font-display text-xl font-bold tracking-tight">Where to go</h2>
      <Cards className="mt-4">
        <Card
          icon={<Activity />}
          title="Monitor a signal"
          href="/docs/monitoring"
          description="Write an STL formula and read the robustness over a trace, online or offline."
        />
        <Card
          icon={<Sigma />}
          title="Verify under noise"
          href="/docs/probabilistic"
          description="Lift noisy readings into a fitted distribution and estimate satisfaction probability."
        />
        <Card
          icon={<Workflow />}
          title="Synthesize a controller"
          href="/docs/synthesis"
          description="Turn a specification into an input sequence or a feedback controller."
        />
        <Card
          icon={<BookOpen />}
          title="Reference"
          href="/docs/reference"
          description="The specification language, the statistical and synthesis methods, and the specifications library."
        />
      </Cards>

      <p className="mt-12 border-t border-fd-border pt-6 text-sm leading-6 text-fd-muted-foreground">
        Built by Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab. Dual licensed under MIT or
        Apache 2.0. Using SENTIL in research? Please{' '}
        <Link href="/docs/reference/methods/citation" prefetch={false} className="text-fd-primary">
          cite it
        </Link>
        .
      </p>
    </div>
  );
}