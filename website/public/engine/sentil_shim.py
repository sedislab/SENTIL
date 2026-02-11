"""The sentil module for the browser playground.

Mirrors the shapes of the real Python binding over the engine that ships on the
page, so a program written here runs unchanged against pip-installed sentil.
The engine is the same Rust core, compiled to WebAssembly.
"""

import json

import js
from pyodide.ffi import to_js


class SentilError(Exception):
    """Base for every error the engine reports."""


class ParseError(SentilError):
    pass


class SemanticError(SentilError):
    pass


class EvaluationError(SentilError):
    pass


def _raise(message):
    low = message.lower()
    if "parse" in low or "unexpected" in low or "expected" in low:
        raise ParseError(message)
    if "unknown variable" in low or "not probabilistic" in low:
        raise SemanticError(message)
    raise EvaluationError(message)


class Trace:
    def __init__(self, times, signals=None):
        self.times = [float(t) for t in times]
        self.signals = {}
        for name, values in (signals or {}).items():
            self.add_signal(name, values)

    def add_signal(self, name, values):
        values = [float(v) for v in values]
        if len(values) != len(self.times):
            raise EvaluationError(
                f"signal '{name}' has {len(values)} samples but the trace has {len(self.times)} timestamps"
            )
        self.signals[name] = values

    def _req(self):
        return {"times": self.times, "signals": self.signals}


class Violation:
    def __init__(self, start, end):
        self.start = start
        self.end = end

    def __repr__(self):
        return f"Violation(start={self.start}, end={self.end})"


class ConfidenceInterval:
    def __init__(self, lower, upper):
        self.lower = lower
        self.upper = upper

    def __repr__(self):
        return f"[{self.lower:.6f}, {self.upper:.6f}]"


class SmcResult:
    def __init__(self, probability, interval, holds):
        self.probability = probability
        self.interval = interval
        self.holds = holds

    def __repr__(self):
        return f"SmcResult(probability={self.probability}, interval={self.interval}, holds={self.holds})"


class SmcConfig:
    def __init__(self, samples=10000, confidence=0.95, seed=42):
        if confidence != 0.95:
            raise EvaluationError(
                "the playground engine runs at the default 0.95 confidence; install sentil to set another level"
            )
        self.samples = int(samples)
        self.seed = int(seed)


class NoiseModel:
    def __init__(self, kind, params):
        self._kind = kind
        self._params = params

    @staticmethod
    def gaussian(mean, std_dev):
        return NoiseModel("gaussian", [float(mean), float(std_dev)])

    @staticmethod
    def uniform(low, high):
        return NoiseModel("uniform", [float(low), float(high)])

    @staticmethod
    def log_normal(mu, sigma):
        return NoiseModel("log_normal", [float(mu), float(sigma)])

    @staticmethod
    def exponential(rate):
        return NoiseModel("exponential", [float(rate)])

    @staticmethod
    def gamma(shape, scale):
        return NoiseModel("gamma", [float(shape), float(scale)])

    @staticmethod
    def beta(alpha, beta):
        return NoiseModel("beta", [float(alpha), float(beta)])


class LiftingRegistry:
    def __init__(self):
        self._entries = []

    def register(self, name, model, interaction="additive"):
        if not isinstance(model, NoiseModel):
            raise EvaluationError("register expects a NoiseModel")
        if interaction not in ("additive", "multiplicative"):
            raise EvaluationError("interaction is 'additive' or 'multiplicative'")
        self._entries.append((name, model, interaction))
        return self


class Formula:
    def __init__(self, source, variables):
        self._source = source
        self.variables = variables

    @staticmethod
    def parse(source):
        reply = json.loads(js.sentil_engine.parse_formula(source))
        if not reply["ok"]:
            raise ParseError(reply["error"])
        return Formula(source, list(reply["variables"]))

    def _evaluate(self, trace, dense):
        req = {"formula": self._source, "dense": dense, **trace._req()}
        reply = json.loads(js.sentil_engine.robustness(json.dumps(req)))
        if not reply["ok"]:
            _raise(reply["error"])
        return reply

    def robustness(self, trace):
        return self._evaluate(trace, False)["value"]

    def robustness_dense(self, trace):
        return self._evaluate(trace, True)["value"]

    def robustness_signal(self, trace):
        return list(self._evaluate(trace, False)["series"])

    def robustness_dense_signal(self, trace):
        return list(self._evaluate(trace, True)["series"])

    def violations(self, trace):
        raw = self._evaluate(trace, False)["violations"]
        return [Violation(a, b) for a, b in raw]

    def check(self, trace, lifting, config=None):
        if not isinstance(lifting, LiftingRegistry) or not lifting._entries:
            raise EvaluationError("check needs a LiftingRegistry with a registered noise model")
        if len(lifting._entries) > 1:
            raise EvaluationError(
                "the playground engine lifts one noisy channel; install sentil for several"
            )
        config = config or SmcConfig()
        name, model, interaction = lifting._entries[0]
        req = {
            "formula": self._source,
            "samples": config.samples,
            "seed": config.seed,
            "noise": {
                "variable": name,
                "kind": model._kind,
                "params": model._params,
                "interaction": interaction,
            },
            **trace._req(),
        }
        reply = json.loads(js.sentil_engine.check_prstl(json.dumps(req)))
        if not reply["ok"]:
            _raise(reply["error"])
        return SmcResult(
            reply["probability"],
            ConfidenceInterval(reply["lo"], reply["hi"]),
            reply["holds"],
        )


class Verdict:
    def __init__(self, value, resolved, satisfied):
        self.value = value
        self.resolved = resolved
        self.satisfied = satisfied

    def __repr__(self):
        return f"Verdict(value={self.value}, resolved={self.resolved}, satisfied={self.satisfied})"


class OnlineMonitor:
    def __init__(self, source):
        parsed = Formula.parse(source)
        self._order = parsed.variables
        try:
            self._inner = js.sentil_engine.StreamMonitor.new(source)
        except Exception as exc:  # a JsException carrying the engine message
            raise ParseError(str(exc)) from None

    def update(self, time, values):
        packed = []
        for name in self._order:
            if name not in values:
                raise EvaluationError(f"update is missing a value for '{name}'")
            packed.append(float(values[name]))
        reply = json.loads(self._inner.update(float(time), to_js(packed)))
        if "error" in reply and reply.get("ok") is False:
            _raise(reply["error"])
        return Verdict(reply["value"], reply["resolved"], reply["satisfied"])

    def reset(self):
        self._inner.reset()


def wilson_interval(successes, trials, level=0.95):
    reply = json.loads(js.sentil_engine.wilson(int(successes), int(trials), float(level)))
    return ConfidenceInterval(reply["lo"], reply["hi"])