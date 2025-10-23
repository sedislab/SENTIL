#ifndef SENTIL_HPP
#define SENTIL_HPP

#include <sentil.h>

#include <cstddef>
#include <cstdint>
#include <map>
#include <optional>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

#include "sentil/errors.hpp"
#include "sentil/types.hpp"

namespace sentil {

namespace detail {

template <typename Fn>
inline bool draw_bernoulli(void* userdata) {
    auto& s = *static_cast<CallbackState<Fn>*>(userdata);
    if (s.error) {
        return false;
    }
    try {
        return s.fn();
    } catch (...) {
        s.error = std::current_exception();
        return false;
    }
}

/// The C create calls signal failure with a null handle.
template <typename T>
inline T* must(T* p) {
    if (!p) {
        raise_last();
    }
    return p;
}

inline std::string owned_string(char* s) {
    if (!s) {
        raise_last();
    }
    std::string out(s);
    sentil_free_string(s);
    return out;
}

/// A null return with no error set is an empty list, not a failure.
inline std::vector<std::string> owned_string_array(char** array, std::size_t count) {
    if (!array) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last();
        }
        return {};
    }
    std::vector<std::string> out;
    out.reserve(count);
    for (std::size_t i = 0; i < count; ++i) {
        out.emplace_back(array[i]);
    }
    sentil_free_string_array(array, count);
    return out;
}

inline std::vector<double> owned_doubles(double* array, std::size_t count) {
    if (!array) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last();
        }
        return {};
    }
    std::vector<double> out(array, array + count);
    sentil_free_doubles(array, count);
    return out;
}

inline std::vector<Interval> owned_intervals(sentil_interval_t* array, std::size_t count) {
    if (!array) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last();
        }
        return {};
    }
    std::vector<Interval> out;
    out.reserve(count);
    for (std::size_t i = 0; i < count; ++i) {
        out.push_back(from_c(array[i]));
    }
    sentil_free_intervals(array, count);
    return out;
}

inline std::pair<std::vector<const char*>, std::vector<double>> unzip(
    const std::map<std::string, double>& values) {
    std::vector<const char*> names;
    std::vector<double> data;
    names.reserve(values.size());
    data.reserve(values.size());
    for (const auto& entry : values) {
        names.push_back(entry.first.c_str());
        data.push_back(entry.second);
    }
    return {std::move(names), std::move(data)};
}

inline std::vector<const char*> c_strs(const std::vector<std::string>& names) {
    std::vector<const char*> out;
    out.reserve(names.size());
    for (const std::string& name : names) {
        out.push_back(name.c_str());
    }
    return out;
}

inline std::vector<std::vector<double>> unflatten(const std::vector<double>& flat, std::size_t rows,
                                                  std::size_t cols) {
    std::vector<std::vector<double>> out;
    out.reserve(rows);
    for (std::size_t i = 0; i < rows; ++i) {
        auto start = flat.begin() + static_cast<std::ptrdiff_t>(i * cols);
        out.emplace_back(start, start + static_cast<std::ptrdiff_t>(cols));
    }
    return out;
}

template <typename HandleT, typename UpdateFn>
inline Robustness update_named(HandleT* handle, UpdateFn update, double time,
                               const std::map<std::string, double>& values) {
    auto [names, data] = unzip(values);
    sentil_robustness_t out;
    ensure(update(handle, time, names.data(), data.data(), names.size(), &out));
    return from_c(out);
}

inline std::vector<Robustness> owned_robustness(sentil_robustness_t* array, std::size_t count) {
    if (!array) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last();
        }
        return {};
    }
    std::vector<Robustness> out;
    out.reserve(count);
    for (std::size_t i = 0; i < count; ++i) {
        out.push_back(from_c(array[i]));
    }
    sentil_free_robustness(array, count);
    return out;
}

inline std::map<std::string, double> bank_results(sentil_bank_result_t* array, std::size_t count) {
    if (!array) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last();
        }
        return {};
    }
    std::map<std::string, double> out;
    bool failed = false;
    std::string failed_id;
    sentil_error_t failed_code = SENTIL_OK;
    for (std::size_t i = 0; i < count; ++i) {
        if (array[i].ok) {
            out.emplace(array[i].id, array[i].value);
        } else if (!failed) {
            failed = true;
            failed_id = array[i].id;
            failed_code = array[i].code;
        }
    }
    sentil_free_bank_results(array, count);
    if (failed) {
        raise_with(failed_code, "formula '" + failed_id + "' failed to evaluate");
    }
    return out;
}

inline std::vector<double> flatten(const std::vector<std::vector<double>>& matrix,
                                   std::size_t expected_cols) {
    std::vector<double> flat;
    flat.reserve(matrix.size() * expected_cols);
    for (const std::vector<double>& row : matrix) {
        if (row.size() != expected_cols) {
            raise_with(SENTIL_ERR_INVALID_CONFIG, "matrix row has the wrong length");
        }
        flat.insert(flat.end(), row.begin(), row.end());
    }
    return flat;
}

template <typename T, void (*Destroy)(T*)>
class Handle {
public:
    Handle() noexcept : ptr_(nullptr) {}
    explicit Handle(T* ptr) noexcept : ptr_(ptr) {}
    ~Handle() {
        if (ptr_) {
            Destroy(ptr_);
        }
    }

    Handle(const Handle&) = delete;
    Handle& operator=(const Handle&) = delete;

    Handle(Handle&& other) noexcept : ptr_(other.ptr_) { other.ptr_ = nullptr; }
    Handle& operator=(Handle&& other) noexcept {
        if (this != &other) {
            if (ptr_) {
                Destroy(ptr_);
            }
            ptr_ = other.ptr_;
            other.ptr_ = nullptr;
        }
        return *this;
    }

    T* get() const noexcept { return ptr_; }
    explicit operator bool() const noexcept { return ptr_ != nullptr; }

    T* release() noexcept {
        T* p = ptr_;
        ptr_ = nullptr;
        return p;
    }

private:
    T* ptr_;
};

}  // namespace detail

/// The version of the linked SENTIL core.
struct Version {
    std::uint32_t major;
    std::uint32_t minor;
    std::uint32_t patch;
};

inline Version version() {
    Version v{0, 0, 0};
    sentil_version(&v.major, &v.minor, &v.patch);
    return v;
}

class Trace;
class LiftingRegistry;
class StochasticSystem;
class SimModel;
class SystemModel;
class Bounds;
struct Witness;

/// A parsed PrSTL formula.
class Formula {
public:
    /// Parse a PrSTL formula.
    static Formula parse(const std::string& text) {
        return Formula(detail::must(sentil_formula_parse(text.c_str())));
    }

    /// Rebuild a formula from the JSON produced by to_json.
    static Formula from_json(const std::string& json) {
        return Formula(detail::must(sentil_formula_from_json(json.c_str())));
    }

    /// The formula as a JSON string.
    std::string to_json() const { return detail::owned_string(sentil_formula_to_json(get())); }

    /// The nesting depth.
    std::size_t depth() const { return sentil_formula_depth(get()); }

    /// Whether the formula uses any temporal operator.
    bool is_temporal() const { return sentil_formula_has_temporal(get()); }

    /// The variable names the formula reads, sorted and unique.
    std::vector<std::string> variables() const {
        std::size_t count = 0;
        char** raw = sentil_formula_variables(get(), &count);
        return detail::owned_string_array(raw, count);
    }

    /// This formula implies consequent.
    Formula implies(Formula consequent) && {
        return Formula(detail::must(sentil_formula_implies(release(), consequent.release())));
    }

    /// The formula holds at the next sample.
    Formula next() && { return Formula(detail::must(sentil_formula_next(release()))); }

    /// The formula holds throughout [lower, upper].
    Formula always(double lower = 0.0, std::optional<double> upper = std::nullopt) && {
        return Formula(detail::must(
            sentil_formula_always(lower, upper.value_or(0.0), upper.has_value(), release())));
    }

    /// The formula holds at some point in [lower, upper].
    Formula eventually(double lower = 0.0, std::optional<double> upper = std::nullopt) && {
        return Formula(detail::must(
            sentil_formula_eventually(lower, upper.value_or(0.0), upper.has_value(), release())));
    }

    /// The formula held throughout the past window [lower, upper].
    Formula historically(double lower = 0.0, std::optional<double> upper = std::nullopt) && {
        return Formula(detail::must(
            sentil_formula_historically(lower, upper.value_or(0.0), upper.has_value(), release())));
    }

    /// The formula held at some past point in [lower, upper].
    Formula once(double lower = 0.0, std::optional<double> upper = std::nullopt) && {
        return Formula(detail::must(
            sentil_formula_once(lower, upper.value_or(0.0), upper.has_value(), release())));
    }

    /// This formula holds until right does, within [lower, upper].
    Formula until(Formula right, double lower = 0.0, std::optional<double> upper = std::nullopt) && {
        return Formula(detail::must(sentil_formula_until(
            lower, upper.value_or(0.0), upper.has_value(), release(), right.release())));
    }

    /// This formula has held since right did, within the past [lower, upper].
    Formula since(Formula right, double lower = 0.0, std::optional<double> upper = std::nullopt) && {
        return Formula(detail::must(sentil_formula_since(
            lower, upper.value_or(0.0), upper.has_value(), release(), right.release())));
    }

    /// Wrap this formula in a probabilistic operator P~p, asserting it holds with
    /// probability op-related to threshold.
    Formula probability(ProbabilityOp op, double threshold) && {
        return Formula(detail::must(sentil_formula_probabilistic(
            static_cast<sentil_probability_op_t>(op), threshold, release())));
    }

    /// The robustness of the formula over the trace, reading the sample grid.
    double robustness(const Trace& trace) const;

    /// The robustness over the trace in dense time.
    double robustness_dense(const Trace& trace) const;

    /// The robustness at every sample, reading the grid.
    std::vector<double> robustness_signal(const Trace& trace) const;

    /// The robustness at every sample in dense time.
    std::vector<double> robustness_dense_signal(const Trace& trace) const;

    /// The time spans where the formula does not hold.
    std::vector<Interval> violations(const Trace& trace) const;

    /// The differentiable surrogate for robustness the synthesis optimizers climb.
    double smooth_robustness(const Trace& trace, const SmoothConfig& config = {}) const;

    /// Estimate the satisfaction probability of this P-wrapped formula by sampling the lifted trace ensemble.
    SmcResult check(const Trace& trace, const LiftingRegistry& lifting,
                    const SmcConfig& config = {}) const;

    /// Like check, but always reports the conservative Clopper-Pearson interval.
    SmcResult check_conservative(const Trace& trace, const LiftingRegistry& lifting,
                                 const SmcConfig& config = {}) const;

    /// Like check, but also reports the robustness distribution across the ensemble.
    std::pair<SmcResult, RobustnessDistribution> check_distribution(
        const Trace& trace, const LiftingRegistry& lifting, const SmcConfig& config = {}) const;

    /// Decide this P-wrapped formula sequentially with Wald's SPRT.
    SprtResult check_sequential(const Trace& trace, const LiftingRegistry& lifting,
                                const SprtConfig& config) const;

    /// Decide this P-wrapped formula with a Bayesian sequential test.
    BayesResult check_bayesian(const Trace& trace, const LiftingRegistry& lifting,
                               const BayesConfig& config) const;

    /// Estimate this P-wrapped formula over a stochastic system by adaptive multilevel splitting.
    RareEventResult check_rare_event(const StochasticSystem& system,
                                     const RareEventConfig& config = {}) const;

    /// Search for a trajectory that violates the formula on the model.
    Witness find_counterexample(const SystemModel& model, const Bounds& bounds,
                                std::size_t max_iters = 200,
                                const SmoothConfig* smooth = nullptr) const;

    /// Search for a violating trajectory globally with restarted CMA-ES on the exact robustness.
    Witness falsify(const SystemModel& model, const Bounds& bounds, const CmaConfig& config = {},
                    std::size_t restarts = 1) const;

    /// Estimate a rare-event probability for this P >= p (always[0, b] psi) formula on the GPU by fixed-effort multilevel splitting.
    GpuSplittingEstimate check_rare_event_gpu(const SimModel& model,
                                              const RareEventConfig& config = {}) const;

    explicit Formula(sentil_formula_t* handle) : handle_(handle) {}

    sentil_formula_t* get() const { return handle_.get(); }

    sentil_formula_t* release() { return handle_.release(); }

private:
    detail::Handle<sentil_formula_t, sentil_formula_destroy> handle_;
};

/// An arithmetic term inside a predicate.
class Expr {
public:
    /// A term that reads the named variable.
    static Expr var(const std::string& name) {
        return Expr(detail::must(sentil_expr_variable(name.c_str())));
    }

    /// A constant term.
    static Expr constant(double value) { return Expr(detail::must(sentil_expr_literal(value))); }

    explicit Expr(sentil_expr_t* handle) : handle_(handle) {}

    sentil_expr_t* get() const { return handle_.get(); }

    sentil_expr_t* release() { return handle_.release(); }

private:
    detail::Handle<sentil_expr_t, sentil_expr_destroy> handle_;
};

namespace detail {

inline Expr make_binary(sentil_binary_op_t op, Expr left, Expr right) {
    return Expr(must(sentil_expr_binary(op, left.release(), right.release())));
}

}  // namespace detail

inline Expr operator+(Expr left, Expr right) {
    return detail::make_binary(SENTIL_BIN_ADD, std::move(left), std::move(right));
}
inline Expr operator-(Expr left, Expr right) {
    return detail::make_binary(SENTIL_BIN_SUB, std::move(left), std::move(right));
}
inline Expr operator*(Expr left, Expr right) {
    return detail::make_binary(SENTIL_BIN_MUL, std::move(left), std::move(right));
}
inline Expr operator/(Expr left, Expr right) {
    return detail::make_binary(SENTIL_BIN_DIV, std::move(left), std::move(right));
}

inline Expr operator+(Expr left, double right) { return std::move(left) + Expr::constant(right); }
inline Expr operator+(double left, Expr right) { return Expr::constant(left) + std::move(right); }
inline Expr operator-(Expr left, double right) { return std::move(left) - Expr::constant(right); }
inline Expr operator-(double left, Expr right) { return Expr::constant(left) - std::move(right); }
inline Expr operator*(Expr left, double right) { return std::move(left) * Expr::constant(right); }
inline Expr operator*(double left, Expr right) { return Expr::constant(left) * std::move(right); }
inline Expr operator/(Expr left, double right) { return std::move(left) / Expr::constant(right); }
inline Expr operator/(double left, Expr right) { return Expr::constant(left) / std::move(right); }

inline Expr operator-(Expr term) { return Expr::constant(0.0) - std::move(term); }

inline Expr abs(Expr term) {
    sentil_expr_t* arg = term.release();
    return Expr(detail::must(sentil_expr_call("abs", &arg, 1)));
}

namespace detail {

inline Formula predicate(Expr left, sentil_comparison_op_t op, Expr right) {
    return Formula(must(sentil_formula_predicate(left.release(), op, right.release())));
}

}  // namespace detail

inline Formula operator>(Expr left, Expr right) {
    return detail::predicate(std::move(left), SENTIL_CMP_GT, std::move(right));
}
inline Formula operator>=(Expr left, Expr right) {
    return detail::predicate(std::move(left), SENTIL_CMP_GE, std::move(right));
}
inline Formula operator<(Expr left, Expr right) {
    return detail::predicate(std::move(left), SENTIL_CMP_LT, std::move(right));
}
inline Formula operator<=(Expr left, Expr right) {
    return detail::predicate(std::move(left), SENTIL_CMP_LE, std::move(right));
}
inline Formula operator==(Expr left, Expr right) {
    return detail::predicate(std::move(left), SENTIL_CMP_EQ, std::move(right));
}
inline Formula operator!=(Expr left, Expr right) {
    return detail::predicate(std::move(left), SENTIL_CMP_NE, std::move(right));
}

inline Formula operator>(Expr left, double right) { return std::move(left) > Expr::constant(right); }
inline Formula operator>(double left, Expr right) { return Expr::constant(left) > std::move(right); }
inline Formula operator>=(Expr left, double right) { return std::move(left) >= Expr::constant(right); }
inline Formula operator>=(double left, Expr right) { return Expr::constant(left) >= std::move(right); }
inline Formula operator<(Expr left, double right) { return std::move(left) < Expr::constant(right); }
inline Formula operator<(double left, Expr right) { return Expr::constant(left) < std::move(right); }
inline Formula operator<=(Expr left, double right) { return std::move(left) <= Expr::constant(right); }
inline Formula operator<=(double left, Expr right) { return Expr::constant(left) <= std::move(right); }
inline Formula operator==(Expr left, double right) { return std::move(left) == Expr::constant(right); }
inline Formula operator==(double left, Expr right) { return Expr::constant(left) == std::move(right); }
inline Formula operator!=(Expr left, double right) { return std::move(left) != Expr::constant(right); }
inline Formula operator!=(double left, Expr right) { return Expr::constant(left) != std::move(right); }

inline Formula operator!(Formula phi) { return Formula(detail::must(sentil_formula_not(phi.release()))); }

inline Formula operator&&(Formula left, Formula right) {
    return Formula(detail::must(sentil_formula_and(left.release(), right.release())));
}

inline Formula operator||(Formula left, Formula right) {
    return Formula(detail::must(sentil_formula_or(left.release(), right.release())));
}

inline Formula implies(Formula antecedent, Formula consequent) {
    return std::move(antecedent).implies(std::move(consequent));
}

/// phi holds at the next sample.
inline Formula next(Formula phi) { return std::move(phi).next(); }

/// phi holds throughout [lower, upper].
inline Formula always(Formula phi, double lower = 0.0, std::optional<double> upper = std::nullopt) {
    return std::move(phi).always(lower, upper);
}

/// phi holds at some point in [lower, upper].
inline Formula eventually(Formula phi, double lower = 0.0,
                          std::optional<double> upper = std::nullopt) {
    return std::move(phi).eventually(lower, upper);
}

/// phi held throughout the past window [lower, upper].
inline Formula historically(Formula phi, double lower = 0.0,
                            std::optional<double> upper = std::nullopt) {
    return std::move(phi).historically(lower, upper);
}

/// phi held at some past point in [lower, upper].
inline Formula once(Formula phi, double lower = 0.0, std::optional<double> upper = std::nullopt) {
    return std::move(phi).once(lower, upper);
}

/// left holds until right does, within [lower, upper].
inline Formula until(Formula left, Formula right, double lower = 0.0,
                     std::optional<double> upper = std::nullopt) {
    return std::move(left).until(std::move(right), lower, upper);
}

/// left has held since right did, within the past [lower, upper].
inline Formula since(Formula left, Formula right, double lower = 0.0,
                     std::optional<double> upper = std::nullopt) {
    return std::move(left).since(std::move(right), lower, upper);
}

/// Wrap phi in a probabilistic operator P~p.
inline Formula probability(Formula phi, ProbabilityOp op, double threshold) {
    return std::move(phi).probability(op, threshold);
}

class PreparedTrace;

/// A multivariate signal.
class Trace {
public:
    /// A trace over the given strictly increasing times, with no signals yet.
    explicit Trace(const std::vector<double>& times)
        : handle_(detail::must(sentil_trace_create(times.data(), times.size()))) {}

    /// A trace over the times carrying one named signal.
    Trace(const std::vector<double>& times, const std::string& name,
          const std::vector<double>& values)
        : handle_(detail::must(sentil_trace_from_signal(times.data(), times.size(), name.c_str(),
                                                        values.data(), values.size()))) {}

    /// A trace over the times carrying the given named signals.
    Trace(const std::vector<double>& times,
          const std::map<std::string, std::vector<double>>& signals)
        : handle_(detail::must(sentil_trace_create(times.data(), times.size()))) {
        add_signals(signals);
    }

    /// A trace with integer times 0, 1, ..., len - 1 and no signals yet.
    static Trace indexed(std::size_t len) { return Trace(detail::must(sentil_trace_indexed(len))); }

    /// Parse a trace from CSV text.
    static Trace from_csv(const std::string& text) {
        return Trace(detail::must(sentil_trace_from_csv(text.c_str())));
    }

    /// Parse a trace from tab-separated text.
    static Trace from_tsv(const std::string& text) {
        return Trace(detail::must(sentil_trace_from_tsv(text.c_str())));
    }

    /// Read a trace from a file, dispatching on its extension.
    static Trace from_path(const std::string& path) {
        return Trace(detail::must(sentil_trace_from_path(path.c_str())));
    }

    /// Add or replace a named signal; its length must equal the trace length.
    void add_signal(const std::string& name, const std::vector<double>& values) {
        ensure(sentil_trace_add_signal(get(), name.c_str(), values.data(), values.size()));
    }

    /// Add or replace several named signals at once.
    void add_signals(const std::map<std::string, std::vector<double>>& signals) {
        for (const auto& entry : signals) {
            add_signal(entry.first, entry.second);
        }
    }

    /// The number of time points.
    std::size_t size() const { return sentil_trace_len(get()); }

    /// Whether the trace has no time points.
    bool empty() const { return sentil_trace_is_empty(get()); }

    /// The time points.
    std::vector<double> times() const {
        std::size_t len = 0;
        const double* p = sentil_trace_times(get(), &len);
        return p ? std::vector<double>(p, p + len) : std::vector<double>();
    }

    /// The signal names, sorted.
    std::vector<std::string> variables() const {
        std::size_t count = 0;
        char** raw = sentil_trace_variables(get(), &count);
        return detail::owned_string_array(raw, count);
    }

    /// The values of a named signal, or empty when the trace has no such signal.
    std::optional<std::vector<double>> signal(const std::string& name) const {
        std::size_t len = 0;
        const double* p = sentil_trace_signal(get(), name.c_str(), &len);
        if (!p) {
            return std::nullopt;
        }
        return std::vector<double>(p, p + len);
    }

    /// Whether the trace carries a signal of this name.
    bool contains(const std::string& name) const {
        std::size_t len = 0;
        return sentil_trace_signal(get(), name.c_str(), &len) != nullptr;
    }

    /// The values of a named signal, throwing std::out_of_range when it is absent.
    std::vector<double> operator[](const std::string& name) const {
        std::optional<std::vector<double>> values = signal(name);
        if (!values) {
            throw std::out_of_range("no signal named '" + name + "' in the trace");
        }
        return *values;
    }

    /// Resample onto new times with the given interpolation.
    Trace resample(const std::vector<double>& times,
                   Interpolation interp = Interpolation::Linear) const {
        return Trace(detail::must(sentil_trace_resample(
            get(), times.data(), times.size(), static_cast<sentil_interpolation_t>(interp))));
    }

    /// Fix the interpolation coefficients for repeated resampling.
    PreparedTrace prepare(Interpolation interp) const;

    explicit Trace(sentil_trace_t* handle) : handle_(handle) {}

    sentil_trace_t* get() const { return handle_.get(); }

    sentil_trace_t* release() { return handle_.release(); }

private:
    detail::Handle<sentil_trace_t, sentil_trace_destroy> handle_;
};

/// A trace with its interpolation coefficients precomputed.
class PreparedTrace {
public:
    /// Resample onto new times using the interpolation fixed at prepare time.
    Trace resample(const std::vector<double>& times) const {
        return Trace(
            detail::must(sentil_prepared_trace_resample(get(), times.data(), times.size())));
    }

    explicit PreparedTrace(sentil_prepared_trace_t* handle) : handle_(handle) {}

    sentil_prepared_trace_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_prepared_trace_t, sentil_prepared_trace_destroy> handle_;
};

inline PreparedTrace Trace::prepare(Interpolation interp) const {
    return PreparedTrace(
        detail::must(sentil_trace_prepare(get(), static_cast<sentil_interpolation_t>(interp))));
}

namespace detail {

inline std::optional<Sample> to_optional(const sentil_sample_t& s) {
    if (!s.found) {
        return std::nullopt;
    }
    return from_c(s);
}

/// The C ABI reports "no estimate yet" as a NaN probability.
inline std::optional<double> to_optional(double p) {
    return std::isnan(p) ? std::nullopt : std::optional<double>(p);
}

}  // namespace detail

/// A fixed-capacity rolling window over the most recent timed samples, keeping
/// running statistics.
class RingBuffer {
public:
    /// A ring buffer holding at most capacity samples.
    explicit RingBuffer(std::size_t capacity)
        : handle_(detail::must(sentil_ring_buffer_create(capacity))) {}

    /// Append a sample, returning the oldest sample if one was evicted.
    std::optional<Sample> push(double time, double value) {
        sentil_sample_t evicted;
        ensure(sentil_ring_buffer_push(get(), time, value, &evicted));
        return detail::to_optional(evicted);
    }

    /// The number of samples currently held.
    std::size_t size() const { return sentil_ring_buffer_len(get()); }

    /// The most samples the buffer can hold.
    std::size_t capacity() const { return sentil_ring_buffer_capacity(get()); }

    /// Whether the buffer holds no samples.
    bool empty() const { return sentil_ring_buffer_is_empty(get()); }

    /// Whether the buffer is at capacity.
    bool is_full() const { return sentil_ring_buffer_is_full(get()); }

    /// Drop every sample.
    void clear() { sentil_ring_buffer_clear(get()); }

    /// The oldest sample, or none when empty.
    std::optional<Sample> front() const {
        return detail::to_optional(sentil_ring_buffer_front(get()));
    }

    /// The newest sample, or none when empty.
    std::optional<Sample> back() const {
        return detail::to_optional(sentil_ring_buffer_back(get()));
    }

    /// The sample at an index counted from the oldest, or none when out of range.
    std::optional<Sample> get(std::size_t index) const {
        return detail::to_optional(sentil_ring_buffer_get(get(), index));
    }

    /// The sample at an index, throwing std::out_of_range when out of range.
    Sample operator[](std::size_t index) const {
        std::optional<Sample> sample = get(index);
        if (!sample) {
            throw std::out_of_range("ring buffer index out of range");
        }
        return *sample;
    }

    /// Remove and return the oldest sample, or none when empty.
    std::optional<Sample> pop_front() {
        return detail::to_optional(sentil_ring_buffer_pop_front(get()));
    }

    /// Remove and return the newest sample, or none when empty.
    std::optional<Sample> pop_back() {
        return detail::to_optional(sentil_ring_buffer_pop_back(get()));
    }

    /// The sample whose time is nearest the query, or none when empty.
    std::optional<Sample> closest_to_time(double time) const {
        return detail::to_optional(sentil_ring_buffer_closest_to_time(get(), time));
    }

    /// The value recorded at the query time within a small tolerance, or none.
    std::optional<double> at_time(double time) const {
        double out;
        return sentil_ring_buffer_at_time(get(), time, &out) ? std::optional<double>(out)
                                                             : std::nullopt;
    }

    /// The earliest and latest times held, or none when empty.
    std::optional<std::pair<double, double>> time_range() const {
        double start;
        double end;
        if (!sentil_ring_buffer_time_range(get(), &start, &end)) {
            return std::nullopt;
        }
        return std::make_pair(start, end);
    }

    /// The samples whose time lies in [start, end], oldest first.
    std::vector<Sample> between(double start, double end) const {
        std::size_t count = 0;
        sentil_sample_t* array = sentil_ring_buffer_between(get(), start, end, &count);
        if (!array) {
            if (sentil_get_last_error_code() != SENTIL_OK) {
                detail::raise_last();
            }
            return {};
        }
        std::vector<Sample> out;
        out.reserve(count);
        for (std::size_t i = 0; i < count; ++i) {
            out.push_back(detail::from_c(array[i]));
        }
        sentil_free_samples(array, count);
        return out;
    }

    /// The mean of the buffered values, or none when empty.
    std::optional<double> mean() const {
        double out;
        return sentil_ring_buffer_mean(get(), &out) ? std::optional<double>(out) : std::nullopt;
    }

    /// The variance of the buffered values, or none when fewer than two are held.
    std::optional<double> variance() const {
        double out;
        return sentil_ring_buffer_variance(get(), &out) ? std::optional<double>(out) : std::nullopt;
    }

    /// The standard deviation, or none when fewer than two values are held.
    std::optional<double> std_dev() const {
        double out;
        return sentil_ring_buffer_std_dev(get(), &out) ? std::optional<double>(out) : std::nullopt;
    }

    /// The smallest buffered value, or none when empty.
    std::optional<double> min() const {
        double out;
        return sentil_ring_buffer_min(get(), &out) ? std::optional<double>(out) : std::nullopt;
    }

    /// The largest buffered value, or none when empty.
    std::optional<double> max() const {
        double out;
        return sentil_ring_buffer_max(get(), &out) ? std::optional<double>(out) : std::nullopt;
    }

    /// Recompute the running mean and variance from scratch.
    void recompute_statistics() { sentil_ring_buffer_recompute_statistics(get()); }

    explicit RingBuffer(sentil_ring_buffer_t* handle) : handle_(handle) {}

    sentil_ring_buffer_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_ring_buffer_t, sentil_ring_buffer_destroy> handle_;
};

inline double Formula::robustness(const Trace& trace) const {
    double out;
    ensure(sentil_formula_robustness(get(), trace.get(), &out));
    return out;
}

inline double Formula::robustness_dense(const Trace& trace) const {
    double out;
    ensure(sentil_formula_robustness_dense(get(), trace.get(), &out));
    return out;
}

inline std::vector<double> Formula::robustness_signal(const Trace& trace) const {
    std::size_t len = 0;
    double* raw = sentil_formula_robustness_signal(get(), trace.get(), &len);
    return detail::owned_doubles(raw, len);
}

inline std::vector<double> Formula::robustness_dense_signal(const Trace& trace) const {
    std::size_t len = 0;
    double* raw = sentil_formula_robustness_dense_signal(get(), trace.get(), &len);
    return detail::owned_doubles(raw, len);
}

inline std::vector<Interval> Formula::violations(const Trace& trace) const {
    std::size_t count = 0;
    sentil_interval_t* raw = sentil_formula_violations(get(), trace.get(), &count);
    return detail::owned_intervals(raw, count);
}

/// The spans where a robustness signal, sampled at the given times, is negative.
inline std::vector<Interval> violation_intervals(const std::vector<double>& times,
                                                 const std::vector<double>& signal) {
    std::size_t count = 0;
    sentil_interval_t* raw =
        sentil_violation_intervals(times.data(), times.size(), signal.data(), signal.size(), &count);
    return detail::owned_intervals(raw, count);
}

inline double Formula::smooth_robustness(const Trace& trace, const SmoothConfig& config) const {
    sentil_smooth_config_t c = detail::to_c(config);
    double out;
    ensure(sentil_formula_smooth_robustness(get(), trace.get(), &c, &out));
    return out;
}

/// A monitor configuration.
class Config {
public:
    explicit Config(TimeMode time = TimeMode::Discrete)
        : handle_(detail::must(sentil_monitor_config_create())) {
        if (time != TimeMode::Discrete) {
            ensure(sentil_monitor_config_set_time(get(), static_cast<sentil_time_mode_t>(time)));
        }
    }

    /// The time mode the monitor will use.
    TimeMode time() const {
        return static_cast<TimeMode>(sentil_monitor_config_time_mode(get()));
    }

    explicit Config(sentil_monitor_config_t* handle) : handle_(handle) {}

    sentil_monitor_config_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_monitor_config_t, sentil_monitor_config_destroy> handle_;
};

/// A monitor for one formula.
class Monitor {
public:
    /// A monitor for a formula, with the default discrete-time config.
    explicit Monitor(Formula formula)
        : handle_(detail::must(sentil_monitor_create(formula.release(), nullptr))) {}

    /// A monitor for a formula with the given config.
    Monitor(Formula formula, const Config& config)
        : handle_(detail::must(sentil_monitor_create(formula.release(), config.get()))) {}

    /// A monitor for a formula string, with the default config.
    explicit Monitor(const std::string& formula)
        : handle_(detail::must(sentil_monitor_parse(formula.c_str(), nullptr))) {}

    /// A monitor for a formula string with the given config.
    Monitor(const std::string& formula, const Config& config)
        : handle_(detail::must(sentil_monitor_parse(formula.c_str(), config.get()))) {}

    /// A copy of the monitored formula.
    Formula formula() const { return Formula(detail::must(sentil_monitor_formula(get()))); }

    /// A copy of the monitor's config.
    Config config() const { return Config(detail::must(sentil_monitor_config(get()))); }

    /// The robustness over the trace, honoring the config's time mode.
    double robustness(const Trace& trace) const {
        double out;
        ensure(sentil_monitor_robustness(get(), trace.get(), &out));
        return out;
    }

    /// The robustness at every sample.
    std::vector<double> robustness_signal(const Trace& trace) const {
        std::size_t len = 0;
        double* raw = sentil_monitor_robustness_signal(get(), trace.get(), &len);
        return detail::owned_doubles(raw, len);
    }

    /// The time spans where the property does not hold.
    std::vector<Interval> violations(const Trace& trace) const {
        std::size_t count = 0;
        sentil_interval_t* raw = sentil_monitor_violations(get(), trace.get(), &count);
        return detail::owned_intervals(raw, count);
    }

    /// The index of a variable in packed-update order, or none when the formula
    /// does not read it.
    std::optional<std::size_t> symbol_index(const std::string& name) {
        std::size_t index = 0;
        bool found = false;
        ensure(sentil_monitor_symbol_index(get(), name.c_str(), &index, &found));
        return found ? std::optional<std::size_t>(index) : std::nullopt;
    }

    /// Fold one timestamped sample given as a map from variable name to value.
    Robustness update(double time, const std::map<std::string, double>& values) {
        return detail::update_named(get(), sentil_monitor_update, time, values);
    }

    /// Fold one sample with values already in symbol_index order.
    Robustness update_packed(double time, const std::vector<double>& values) {
        sentil_robustness_t out;
        ensure(sentil_monitor_update_packed(get(), time, values.data(), values.size(), &out));
        return detail::from_c(out);
    }

    /// Clear streaming state so the monitor can run a fresh trace.
    void reset() { sentil_monitor_reset(get()); }

    /// Check this monitor's probabilistic formula using its configured SMC
    /// settings and the lifted trace ensemble.
    SmcResult check(const Trace& trace, const LiftingRegistry& lifting) const;

    /// Decide this monitor's probabilistic formula sequentially with Wald's SPRT.
    SprtResult check_sequential(const Trace& trace, const LiftingRegistry& lifting,
                                const SprtConfig& config) const;

    /// Estimate this monitor's probabilistic formula over a stochastic system by
    /// rare-event splitting.
    RareEventResult check_rare(const StochasticSystem& system) const;

    explicit Monitor(sentil_monitor_t* handle) : handle_(handle) {}

    sentil_monitor_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_monitor_t, sentil_monitor_destroy> handle_;
};

/// The streaming monitor.
class OnlineMonitor {
public:
    /// A streaming monitor for a formula string.
    explicit OnlineMonitor(const std::string& formula)
        : handle_(detail::must(sentil_stream_monitor_create(formula.c_str()))) {}

    /// A streaming monitor for a formula.
    explicit OnlineMonitor(const Formula& formula)
        : handle_(detail::must(sentil_stream_monitor_from_formula(formula.get()))) {}

    /// A streaming monitor that tracks a P~p(phi) formula online, lifting each
    /// reading into a particle ensemble through the registry.
    static OnlineMonitor with_lifting(const Formula& formula, const LiftingRegistry& lifting,
                                      const SmcConfig& config = {});

    /// The number of variables the formula reads.
    std::size_t variable_count() const { return sentil_stream_monitor_variable_count(get()); }

    /// The index of a variable in packed-update order, or none when the formula
    /// does not read it.
    std::optional<std::size_t> symbol_index(const std::string& name) const {
        std::size_t index = 0;
        bool found = false;
        ensure(sentil_stream_monitor_symbol_index(get(), name.c_str(), &index, &found));
        return found ? std::optional<std::size_t>(index) : std::nullopt;
    }

    /// Fold one timestamped sample given as a map from variable name to value.
    Robustness update(double time, const std::map<std::string, double>& values) {
        return detail::update_named(get(), sentil_stream_monitor_update, time, values);
    }

    /// Fold one sample with values already in symbol_index order.
    Robustness update_packed(double time, const std::vector<double>& values) {
        sentil_robustness_t out;
        ensure(sentil_stream_monitor_update_packed(get(), time, values.data(), values.size(), &out));
        return detail::from_c(out);
    }

    /// Replay a whole trace, returning the per-sample robustness.
    std::vector<Robustness> run(const Trace& trace) {
        std::size_t count = 0;
        sentil_robustness_t* raw = sentil_stream_monitor_run(get(), trace.get(), &count);
        return detail::owned_robustness(raw, count);
    }

    /// Clear streaming state so the monitor can run a fresh trace.
    void reset() { sentil_stream_monitor_reset(get()); }

    explicit OnlineMonitor(sentil_stream_monitor_t* handle) : handle_(handle) {}

    sentil_stream_monitor_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_stream_monitor_t, sentil_stream_monitor_destroy> handle_;
};

/// Several streaming formulas under one clock.
class MultiMonitor {
public:
    MultiMonitor() : handle_(detail::must(sentil_multi_monitor_create())) {}

    /// Add a formula string under an id.
    void add(const std::string& id, const std::string& formula) {
        ensure(sentil_multi_monitor_add(get(), id.c_str(), formula.c_str()));
    }

    /// Add a formula under an id.
    void add(const std::string& id, const Formula& formula) {
        ensure(sentil_multi_monitor_add_formula(get(), id.c_str(), formula.get()));
    }

    /// Remove the first formula with the id, returning whether one was found.
    bool remove(const std::string& id) { return sentil_multi_monitor_remove(get(), id.c_str()); }

    /// Clear every monitor's streaming state.
    void reset() { sentil_multi_monitor_reset(get()); }

    /// The number of formulas.
    std::size_t size() const { return sentil_multi_monitor_len(get()); }

    /// Whether no formula is registered.
    bool empty() const { return sentil_multi_monitor_is_empty(get()); }

    /// The ids in insertion order.
    std::vector<std::string> ids() const {
        std::size_t count = 0;
        char** raw = sentil_multi_monitor_ids(get(), &count);
        return detail::owned_string_array(raw, count);
    }

    /// Add a P~p(phi) formula tracked online through a lifted particle ensemble.
    void add_probabilistic(const std::string& id, const Formula& formula,
                           const LiftingRegistry& lifting, const SmcConfig& config = {});

    /// Advance every monitor at this sample, returning the verdict for each id.
    std::map<std::string, Robustness> update(double time,
                                             const std::map<std::string, double>& values) {
        auto [names, data] = detail::unzip(values);
        std::size_t count = 0;
        sentil_named_robustness_t* raw = sentil_multi_monitor_update(get(), time, names.data(),
                                                                     data.data(), names.size(),
                                                                     &count);
        if (!raw) {
            if (sentil_get_last_error_code() != SENTIL_OK) {
                detail::raise_last();
            }
            return {};
        }
        std::map<std::string, Robustness> out;
        for (std::size_t i = 0; i < count; ++i) {
            out.emplace(raw[i].id, detail::from_c(raw[i].robustness));
        }
        sentil_free_named_robustness(raw, count);
        return out;
    }

    explicit MultiMonitor(sentil_multi_monitor_t* handle) : handle_(handle) {}

    sentil_multi_monitor_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_multi_monitor_t, sentil_multi_monitor_destroy> handle_;
};

/// A batch of named formulas evaluated together over one trace.
class FormulaBank {
public:
    FormulaBank() : handle_(detail::must(sentil_formula_bank_create())) {}

    /// Add a formula string under an id.
    void add(const std::string& id, const std::string& formula) {
        ensure(sentil_formula_bank_add(get(), id.c_str(), formula.c_str()));
    }

    /// Add a formula under an id.
    void add(const std::string& id, const Formula& formula) {
        ensure(sentil_formula_bank_add_formula(get(), id.c_str(), formula.get()));
    }

    /// The ids in insertion order.
    std::vector<std::string> ids() const {
        std::size_t count = 0;
        char** raw = sentil_formula_bank_ids(get(), &count);
        return detail::owned_string_array(raw, count);
    }

    /// The number of formulas.
    std::size_t size() const { return sentil_formula_bank_len(get()); }

    /// Whether no formula is registered.
    bool empty() const { return sentil_formula_bank_is_empty(get()); }

    /// The robustness of every formula over the trace, keyed by id.
    std::map<std::string, double> robustness(const Trace& trace) const {
        std::size_t count = 0;
        sentil_bank_result_t* raw = sentil_formula_bank_robustness(get(), trace.get(), &count);
        return detail::bank_results(raw, count);
    }

    /// The dense-time robustness of every formula over the trace, keyed by id.
    std::map<std::string, double> robustness_dense(const Trace& trace) const {
        std::size_t count = 0;
        sentil_bank_result_t* raw = sentil_formula_bank_robustness_dense(get(), trace.get(), &count);
        return detail::bank_results(raw, count);
    }

    explicit FormulaBank(sentil_formula_bank_t* handle) : handle_(handle) {}

    sentil_formula_bank_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_formula_bank_t, sentil_formula_bank_destroy> handle_;
};

/// A noise distribution for stochastic signal lifting.
class NoiseModel {
public:
    /// A point mass at value.
    static NoiseModel dirac(double value) {
        return NoiseModel(detail::must(sentil_noise_dirac(value)));
    }
    /// A normal distribution.
    static NoiseModel gaussian(double mean, double std_dev) {
        return NoiseModel(detail::must(sentil_noise_gaussian(mean, std_dev)));
    }
    /// A uniform distribution on [low, high].
    static NoiseModel uniform(double low, double high) {
        return NoiseModel(detail::must(sentil_noise_uniform(low, high)));
    }
    /// A log-normal distribution.
    static NoiseModel log_normal(double mu, double sigma) {
        return NoiseModel(detail::must(sentil_noise_log_normal(mu, sigma)));
    }
    /// An exponential distribution with the given rate.
    static NoiseModel exponential(double rate) {
        return NoiseModel(detail::must(sentil_noise_exponential(rate)));
    }
    /// A gamma distribution.
    static NoiseModel gamma(double shape, double scale) {
        return NoiseModel(detail::must(sentil_noise_gamma(shape, scale)));
    }
    /// A beta distribution.
    static NoiseModel beta(double alpha, double beta) {
        return NoiseModel(detail::must(sentil_noise_beta(alpha, beta)));
    }
    /// A Weibull distribution.
    static NoiseModel weibull(double shape, double scale) {
        return NoiseModel(detail::must(sentil_noise_weibull(shape, scale)));
    }
    /// A Rayleigh distribution.
    static NoiseModel rayleigh(double scale) {
        return NoiseModel(detail::must(sentil_noise_rayleigh(scale)));
    }
    /// A Gumbel distribution.
    static NoiseModel gumbel(double location, double scale) {
        return NoiseModel(detail::must(sentil_noise_gumbel(location, scale)));
    }
    /// A Cauchy distribution.
    static NoiseModel cauchy(double location, double scale) {
        return NoiseModel(detail::must(sentil_noise_cauchy(location, scale)));
    }
    /// A Student's t distribution.
    static NoiseModel student_t(double df, double location, double scale) {
        return NoiseModel(detail::must(sentil_noise_student_t(df, location, scale)));
    }
    /// A normal distribution truncated to [lower, upper].
    static NoiseModel truncated_normal(double mean, double std_dev, double lower, double upper) {
        return NoiseModel(detail::must(sentil_noise_truncated_normal(mean, std_dev, lower, upper)));
    }
    /// A Poisson distribution with the given rate.
    static NoiseModel poisson(double rate) {
        return NoiseModel(detail::must(sentil_noise_poisson(rate)));
    }
    /// A binomial distribution of n trials with success probability p.
    static NoiseModel binomial(std::uint64_t n, double p) {
        return NoiseModel(detail::must(sentil_noise_binomial(n, p)));
    }
    /// An empirical model resampled from residuals.
    static NoiseModel bootstrap(const std::vector<double>& residuals) {
        return NoiseModel(detail::must(sentil_noise_bootstrap(residuals.data(), residuals.size())));
    }

    /// A weighted mixture of component models.
    static NoiseModel mixture(const std::vector<double>& weights, std::vector<NoiseModel> models) {
        if (weights.size() != models.size()) {
            detail::raise_with(SENTIL_ERR_INVALID_CONFIG,
                               "a mixture needs one weight per component model");
        }
        std::vector<sentil_noise_model_t*> raw;
        raw.reserve(models.size());
        for (NoiseModel& model : models) {
            raw.push_back(model.release());
        }
        return NoiseModel(detail::must(sentil_noise_mixture(weights.data(), raw.data(), raw.size())));
    }

    /// A weighted mixture of components passed inline.
    template <typename... Models,
              std::enable_if_t<(... && std::is_same_v<std::decay_t<Models>, NoiseModel>), int> = 0>
    static NoiseModel mixture(const std::vector<double>& weights, Models&&... models) {
        std::vector<NoiseModel> collected;
        collected.reserve(sizeof...(models));
        (collected.push_back(std::move(models)), ...);
        return mixture(weights, std::move(collected));
    }

    /// A maximum-likelihood Gaussian fit of the samples.
    static NoiseModel fit_gaussian(const std::vector<double>& samples) {
        return NoiseModel(detail::must(sentil_noise_fit_gaussian(samples.data(), samples.size())));
    }

    /// The empirical bootstrap of the samples.
    static NoiseModel fit_bootstrap(const std::vector<double>& samples) {
        return NoiseModel(detail::must(sentil_noise_fit_bootstrap(samples.data(), samples.size())));
    }

    /// A reservoir-sampled bootstrap that caps the retained residuals.
    static NoiseModel fit_bootstrap_reservoir(const std::vector<double>& samples,
                                              std::size_t max_samples) {
        return NoiseModel(detail::must(
            sentil_noise_fit_bootstrap_reservoir(samples.data(), samples.size(), max_samples)));
    }

    /// A Gaussian mixture fit by expectation-maximization.
    static NoiseModel fit_gaussian_mixture(const std::vector<double>& samples,
                                           std::size_t components, std::size_t max_iters) {
        return NoiseModel(detail::must(sentil_noise_fit_gaussian_mixture(
            samples.data(), samples.size(), components, max_iters)));
    }

    /// The residuals between paired ground-truth and sensor readings.
    static std::vector<double> residuals(const std::vector<double>& ground_truth,
                                         const std::vector<double>& sensor,
                                         NoiseInteraction interaction) {
        std::size_t len = 0;
        double* raw = sentil_noise_residuals(ground_truth.data(), ground_truth.size(), sensor.data(),
                                             sensor.size(),
                                             static_cast<sentil_noise_interaction_t>(interaction),
                                             &len);
        return detail::owned_doubles(raw, len);
    }

    /// The analytic mean, or none where it is undefined.
    std::optional<double> mean() const {
        double out;
        return sentil_noise_mean(get(), &out) ? std::optional<double>(out) : std::nullopt;
    }

    /// The analytic variance, or none where it is undefined.
    std::optional<double> variance() const {
        double out;
        return sentil_noise_variance(get(), &out) ? std::optional<double>(out) : std::nullopt;
    }

    /// The model as a JSON string.
    std::string to_json() const { return detail::owned_string(sentil_noise_to_json(get())); }

    /// Rebuild a model from the JSON produced by to_json.
    static NoiseModel from_json(const std::string& json) {
        return NoiseModel(detail::must(sentil_noise_from_json(json.c_str())));
    }

    /// Load a model from a JSON file.
    static NoiseModel from_file(const std::string& path) {
        return NoiseModel(detail::must(sentil_noise_from_file(path.c_str())));
    }

    explicit NoiseModel(sentil_noise_model_t* handle) : handle_(handle) {}

    sentil_noise_model_t* get() const { return handle_.get(); }

    sentil_noise_model_t* release() { return handle_.release(); }

private:
    detail::Handle<sentil_noise_model_t, sentil_noise_destroy> handle_;
};

/// The per-variable noise models that turn a deterministic trace into a stochastic ensemble.
class LiftingRegistry {
public:
    LiftingRegistry() : handle_(detail::must(sentil_lifting_registry_create())) {}

    /// Attach a noise model to a variable.
    void register_noise(const std::string& variable, NoiseModel model,
                        NoiseInteraction interaction = NoiseInteraction::Additive) {
        ensure(sentil_lifting_registry_register(get(), variable.c_str(), model.release(),
                                               static_cast<sentil_noise_interaction_t>(interaction)));
    }

    /// The variables that carry a noise model, sorted.
    std::vector<std::string> variables() const {
        std::size_t count = 0;
        char** raw = sentil_lifting_registry_variables(get(), &count);
        return detail::owned_string_array(raw, count);
    }

    /// Whether no variable carries a noise model.
    bool empty() const { return sentil_lifting_registry_is_empty(get()); }

    /// One seeded noisy realization of the trace.
    Trace lift(const Trace& trace, std::uint64_t seed = 42) const {
        return Trace(detail::must(sentil_lifting_registry_lift(get(), trace.get(), seed)));
    }

    explicit LiftingRegistry(sentil_lifting_registry_t* handle) : handle_(handle) {}

    sentil_lifting_registry_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_lifting_registry_t, sentil_lifting_registry_destroy> handle_;
};

inline OnlineMonitor OnlineMonitor::with_lifting(const Formula& formula,
                                                const LiftingRegistry& lifting,
                                                const SmcConfig& config) {
    sentil_smc_config_t c = detail::to_c(config);
    return OnlineMonitor(
        detail::must(sentil_stream_monitor_with_lifting(formula.get(), lifting.get(), &c)));
}

inline void MultiMonitor::add_probabilistic(const std::string& id, const Formula& formula,
                                            const LiftingRegistry& lifting,
                                            const SmcConfig& config) {
    sentil_smc_config_t c = detail::to_c(config);
    ensure(sentil_multi_monitor_add_probabilistic(get(), id.c_str(), formula.get(), lifting.get(),
                                                  &c));
}

inline SmcResult Formula::check(const Trace& trace, const LiftingRegistry& lifting,
                                const SmcConfig& config) const {
    sentil_smc_config_t c = detail::to_c(config);
    sentil_smc_result_t out;
    ensure(sentil_formula_check(get(), trace.get(), lifting.get(), &c, &out));
    return detail::from_c(out);
}

inline SmcResult Formula::check_conservative(const Trace& trace, const LiftingRegistry& lifting,
                                             const SmcConfig& config) const {
    sentil_smc_config_t c = detail::to_c(config);
    sentil_smc_result_t out;
    ensure(sentil_formula_check_conservative(get(), trace.get(), lifting.get(), &c, &out));
    return detail::from_c(out);
}

inline std::pair<SmcResult, RobustnessDistribution> Formula::check_distribution(
    const Trace& trace, const LiftingRegistry& lifting, const SmcConfig& config) const {
    sentil_smc_config_t c = detail::to_c(config);
    sentil_smc_result_t result;
    sentil_robustness_distribution_t distribution;
    ensure(sentil_formula_check_distribution(get(), trace.get(), lifting.get(), &c, &result,
                                                    &distribution));
    return {detail::from_c(result), detail::from_c(distribution)};
}

inline SmcResult Monitor::check(const Trace& trace, const LiftingRegistry& lifting) const {
    sentil_smc_result_t out;
    ensure(sentil_monitor_check(get(), trace.get(), lifting.get(), &out));
    return detail::from_c(out);
}

inline SprtResult Formula::check_sequential(const Trace& trace, const LiftingRegistry& lifting,
                                            const SprtConfig& config) const {
    sentil_sprt_config_t c = detail::to_c(config);
    sentil_sprt_result_t out;
    ensure(sentil_formula_check_sequential(get(), trace.get(), lifting.get(), &c, &out));
    return detail::from_c(out);
}

inline BayesResult Formula::check_bayesian(const Trace& trace, const LiftingRegistry& lifting,
                                           const BayesConfig& config) const {
    sentil_bayes_config_t c = detail::to_c(config);
    sentil_bayes_result_t out;
    ensure(sentil_formula_check_bayesian(get(), trace.get(), lifting.get(), &c, &out));
    return detail::from_c(out);
}

inline SprtResult Monitor::check_sequential(const Trace& trace, const LiftingRegistry& lifting,
                                            const SprtConfig& config) const {
    sentil_sprt_config_t c = detail::to_c(config);
    sentil_sprt_result_t out;
    ensure(sentil_monitor_check_sequential(get(), trace.get(), lifting.get(), &c, &out));
    return detail::from_c(out);
}

/// Binomial proportion confidence intervals and the sample-size formulas.
namespace stats {

/// The Wilson score interval for successes out of trials at the confidence level.
inline ConfidenceInterval wilson_interval(std::uint64_t successes, std::uint64_t trials,
                                          double level) {
    return detail::from_c(sentil_wilson_interval(successes, trials, level));
}

/// The Clopper-Pearson exact interval.
inline ConfidenceInterval clopper_pearson(std::uint64_t successes, std::uint64_t trials,
                                          double level) {
    return detail::from_c(sentil_clopper_pearson(successes, trials, level));
}

/// The Jeffreys interval.
inline ConfidenceInterval jeffreys_interval(std::uint64_t successes, std::uint64_t trials,
                                            double level) {
    return detail::from_c(sentil_jeffreys_interval(successes, trials, level));
}

/// The Agresti-Coull interval.
inline ConfidenceInterval agresti_coull(std::uint64_t successes, std::uint64_t trials,
                                        double level) {
    return detail::from_c(sentil_agresti_coull(successes, trials, level));
}

/// A confidence interval by the chosen method.
inline ConfidenceInterval interval(std::uint64_t successes, std::uint64_t trials, double level,
                                   IntervalMethod method = IntervalMethod::Wilson) {
    return detail::from_c(sentil_interval(static_cast<sentil_interval_method_t>(method), successes,
                                          trials, level));
}

/// The two-sided z critical value for a confidence level in (0, 1).
inline double z_score(double level) { return sentil_z_score(level); }

/// The sample count the Chernoff-Hoeffding bound needs for a target error and
/// confidence.
inline std::uint64_t chernoff_hoeffding_samples(double epsilon, double delta) {
    std::uint64_t out;
    ensure(sentil_chernoff_hoeffding_samples(epsilon, delta, &out));
    return out;
}

/// The sample count for a target half-width and confidence under the Wilson
/// interval.
inline std::uint64_t wilson_samples(double epsilon, double level) {
    std::uint64_t out;
    ensure(sentil_wilson_samples(epsilon, level, &out));
    return out;
}

}  // namespace stats

/// A term in a declarative stochastic update.
class SimExpr {
public:
    /// The previous step's value of the variable at this index.
    static SimExpr prev(std::size_t variable) {
        return SimExpr(detail::must(sentil_sim_expr_prev(variable)));
    }
    /// The current time.
    static SimExpr time() { return SimExpr(detail::must(sentil_sim_expr_time())); }
    /// A constant.
    static SimExpr constant(double value) {
        return SimExpr(detail::must(sentil_sim_expr_const(value)));
    }
    /// A draw from the noise source at this index.
    static SimExpr noise(std::size_t source) {
        return SimExpr(detail::must(sentil_sim_expr_noise(source)));
    }

    explicit SimExpr(sentil_sim_expr_t* handle) : handle_(handle) {}

    sentil_sim_expr_t* get() const { return handle_.get(); }

    sentil_sim_expr_t* release() { return handle_.release(); }

private:
    detail::Handle<sentil_sim_expr_t, sentil_sim_expr_destroy> handle_;
};

inline SimExpr operator+(SimExpr left, SimExpr right) {
    return SimExpr(detail::must(sentil_sim_expr_add(left.release(), right.release())));
}
inline SimExpr operator-(SimExpr left, SimExpr right) {
    return SimExpr(detail::must(sentil_sim_expr_sub(left.release(), right.release())));
}
inline SimExpr operator*(SimExpr left, SimExpr right) {
    return SimExpr(detail::must(sentil_sim_expr_mul(left.release(), right.release())));
}
inline SimExpr operator/(SimExpr left, SimExpr right) {
    return SimExpr(detail::must(sentil_sim_expr_div(left.release(), right.release())));
}

inline SimExpr operator+(SimExpr left, double right) {
    return std::move(left) + SimExpr::constant(right);
}
inline SimExpr operator+(double left, SimExpr right) {
    return SimExpr::constant(left) + std::move(right);
}
inline SimExpr operator-(SimExpr left, double right) {
    return std::move(left) - SimExpr::constant(right);
}
inline SimExpr operator-(double left, SimExpr right) {
    return SimExpr::constant(left) - std::move(right);
}
inline SimExpr operator*(SimExpr left, double right) {
    return std::move(left) * SimExpr::constant(right);
}
inline SimExpr operator*(double left, SimExpr right) {
    return SimExpr::constant(left) * std::move(right);
}
inline SimExpr operator/(SimExpr left, double right) {
    return std::move(left) / SimExpr::constant(right);
}
inline SimExpr operator/(double left, SimExpr right) {
    return SimExpr::constant(left) / std::move(right);
}

inline SimExpr operator-(SimExpr term) { return SimExpr::constant(0.0) - std::move(term); }

/// A sampling-ready stochastic system, the form the rare-event estimator consumes.
/// Build one from a SimModel with to_stochastic_system.
class StochasticSystem {
public:
    /// Simulate one full-horizon trajectory from a seed.
    Trace simulate(std::uint64_t seed = 42) const {
        return Trace(detail::must(sentil_stochastic_system_simulate(get(), seed)));
    }

    /// The state variable names.
    std::vector<std::string> variables() const {
        std::size_t count = 0;
        char** raw = sentil_stochastic_system_variables(get(), &count);
        return detail::owned_string_array(raw, count);
    }

    /// The time step.
    double dt() const { return sentil_stochastic_system_dt(get()); }

    /// The number of steps in a trajectory.
    std::size_t horizon() const { return sentil_stochastic_system_horizon(get()); }

    explicit StochasticSystem(sentil_stochastic_system_t* handle) : handle_(handle) {}

    sentil_stochastic_system_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_stochastic_system_t, sentil_stochastic_system_destroy> handle_;
};

/// A declarative stochastic model.
class SimModel {
public:
    SimModel(const std::vector<std::string>& variables, double dt, std::size_t horizon,
             std::vector<SimExpr> init, std::vector<SimExpr> advance, std::vector<NoiseModel> noise)
        : handle_(build(variables, dt, horizon, init, advance, noise)) {}

    /// Simulate one full-horizon trajectory from a seed.
    Trace simulate(std::uint64_t seed = 42) const {
        return Trace(detail::must(sentil_sim_model_simulate(get(), seed)));
    }

    /// Convert to a stochastic system for the rare-event path.
    StochasticSystem to_stochastic_system() const {
        return StochasticSystem(detail::must(sentil_sim_model_to_stochastic_system(get())));
    }

    /// The variable names.
    std::vector<std::string> variables() const {
        std::size_t count = 0;
        char** raw = sentil_sim_model_variables(get(), &count);
        return detail::owned_string_array(raw, count);
    }

    /// The time step.
    double dt() const { return sentil_sim_model_dt(get()); }

    /// The number of steps in a trajectory.
    std::size_t horizon() const { return sentil_sim_model_horizon(get()); }

    explicit SimModel(sentil_sim_model_t* handle) : handle_(handle) {}

    sentil_sim_model_t* get() const { return handle_.get(); }

private:
    static sentil_sim_model_t* build(const std::vector<std::string>& variables, double dt,
                                     std::size_t horizon, std::vector<SimExpr>& init,
                                     std::vector<SimExpr>& advance, std::vector<NoiseModel>& noise) {
        std::vector<const char*> names = detail::c_strs(variables);
        std::vector<sentil_sim_expr_t*> init_raw;
        std::vector<sentil_sim_expr_t*> advance_raw;
        std::vector<sentil_noise_model_t*> noise_raw;
        init_raw.reserve(init.size());
        advance_raw.reserve(advance.size());
        noise_raw.reserve(noise.size());
        for (SimExpr& expr : init) {
            init_raw.push_back(expr.release());
        }
        for (SimExpr& expr : advance) {
            advance_raw.push_back(expr.release());
        }
        for (NoiseModel& model : noise) {
            noise_raw.push_back(model.release());
        }
        return detail::must(sentil_sim_model_create(
            names.data(), names.size(), dt, horizon, init_raw.data(), init_raw.size(),
            advance_raw.data(), advance_raw.size(), noise_raw.data(), noise_raw.size()));
    }

    detail::Handle<sentil_sim_model_t, sentil_sim_model_destroy> handle_;
};

/// Per-coordinate box bounds on a synthesis input.
class Bounds {
public:
    /// Bounds with the given lower and upper limit per coordinate.
    Bounds(const std::vector<double>& lower, const std::vector<double>& upper)
        : handle_(make(lower, upper)) {}

    /// Bounds that constrain nothing over the given number of coordinates.
    static Bounds unbounded(std::size_t dimension) {
        return Bounds(detail::must(sentil_bounds_unbounded(dimension)));
    }

    /// The number of coordinates.
    std::size_t dimension() const { return sentil_bounds_dimension(get()); }

    /// The per-coordinate lower limits.
    std::vector<double> lower() const {
        std::vector<double> out(dimension());
        sentil_bounds_lower(get(), out.data());
        return out;
    }

    /// The per-coordinate upper limits.
    std::vector<double> upper() const {
        std::vector<double> out(dimension());
        sentil_bounds_upper(get(), out.data());
        return out;
    }

    /// Project a point into the box, returning the clamped copy.
    std::vector<double> clamp(std::vector<double> point) const {
        if (point.size() != dimension()) {
            detail::raise_with(SENTIL_ERR_INVALID_CONFIG,
                               "point length must equal the bounds dimension");
        }
        sentil_bounds_clamp(get(), point.data(), point.size());
        return point;
    }

    explicit Bounds(sentil_bounds_t* handle) : handle_(handle) {}

    sentil_bounds_t* get() const { return handle_.get(); }

    sentil_bounds_t* release() { return handle_.release(); }

private:
    static sentil_bounds_t* make(const std::vector<double>& lower,
                                 const std::vector<double>& upper) {
        if (lower.size() != upper.size()) {
            detail::raise_with(SENTIL_ERR_INVALID_CONFIG,
                               "bounds lower and upper must have the same length");
        }
        return detail::must(sentil_bounds_create(lower.data(), upper.data(), lower.size()));
    }

    detail::Handle<sentil_bounds_t, sentil_bounds_destroy> handle_;
};

/// A dynamical system the synthesizer drives. The only public constructor is the
/// linear time-invariant model; closure-driven models are out of scope, as in the
/// other bindings, because the engine would run the callback across worker threads.
class SystemModel {
public:
    /// A linear model x_{t+1} = A x_t + B u_t.
    static SystemModel linear(const std::vector<std::vector<double>>& a,
                              const std::vector<std::vector<double>>& b,
                              const std::vector<double>& x0,
                              const std::vector<std::string>& variables, double dt,
                              std::size_t horizon) {
        std::size_t n = a.size();
        if (x0.size() != n || variables.size() != n || b.size() != n) {
            detail::raise_with(SENTIL_ERR_INVALID_CONFIG,
                               "A is n-by-n, B has n rows, and x0 and variables have length n");
        }
        std::size_t b_cols = b.empty() ? 0 : b.front().size();
        std::vector<double> a_flat = detail::flatten(a, n);
        std::vector<double> b_flat = detail::flatten(b, b_cols);
        std::vector<const char*> names = detail::c_strs(variables);
        return SystemModel(detail::must(sentil_linear_model_create(
            a_flat.data(), n, b_flat.data(), b_cols, x0.data(), names.data(), names.size(), dt,
            horizon)));
    }

    /// The total length of the input sequence the synthesizer optimizes.
    std::size_t input_dimension() const { return sentil_system_model_input_dimension(get()); }

    explicit SystemModel(sentil_system_model_t* handle) : handle_(handle) {}

    sentil_system_model_t* get() const { return handle_.get(); }

    sentil_system_model_t* release() { return handle_.release(); }

private:
    detail::Handle<sentil_system_model_t, sentil_system_model_destroy> handle_;
};

/// The result of open-loop synthesis.
struct SynthesisResult {
    std::vector<double> input;
    double robustness;
    bool holds;
    Backend backend;
};

/// Smooth-robustness primitives and the synthesis numerics.
namespace synthesis {

/// The smooth (differentiable) minimum of values at the given temperature.
inline double soft_min(const std::vector<double>& values, double temperature) {
    return sentil_soft_min(values.data(), values.size(), temperature);
}

/// The smooth (differentiable) maximum of values at the given temperature.
inline double soft_max(const std::vector<double>& values, double temperature) {
    return sentil_soft_max(values.data(), values.size(), temperature);
}

/// Minimize 1/2 u'Pu + q'u subject to Gu <= h.
inline std::vector<double> solve_qp(const std::vector<std::vector<double>>& p,
                                    const std::vector<double>& q,
                                    const std::vector<std::vector<double>>& g,
                                    const std::vector<double>& h, std::size_t max_iters = 200) {
    std::size_t n = p.size();
    std::size_t m = g.size();
    if (q.size() != n) {
        detail::raise_with(SENTIL_ERR_INVALID_CONFIG, "q must have one entry per column of P");
    }
    if (h.size() != m) {
        detail::raise_with(SENTIL_ERR_INVALID_CONFIG, "h must have one entry per row of G");
    }
    std::vector<double> p_flat = detail::flatten(p, n);
    std::vector<double> g_flat = detail::flatten(g, n);
    std::vector<double> out(n);
    ensure(sentil_solve_qp(p_flat.data(), n, q.data(), g_flat.data(), m, h.data(), max_iters,
                           out.data()));
    return out;
}

/// Solve Ax = b for a symmetric positive-definite A.
inline std::vector<double> solve_spd(const std::vector<std::vector<double>>& matrix,
                                     const std::vector<double>& rhs) {
    std::size_t n = matrix.size();
    if (rhs.size() != n) {
        detail::raise_with(SENTIL_ERR_INVALID_CONFIG, "the right-hand side must match A's order");
    }
    std::vector<double> flat = detail::flatten(matrix, n);
    std::vector<double> out(n);
    ensure(sentil_solve_spd(flat.data(), n, rhs.data(), out.data()));
    return out;
}

/// The eigenvalues and eigenvectors of a symmetric matrix, the eigenvectors as rows.
inline std::pair<std::vector<double>, std::vector<std::vector<double>>> symmetric_eigen(
    const std::vector<std::vector<double>>& matrix) {
    std::size_t n = matrix.size();
    std::vector<double> flat = detail::flatten(matrix, n);
    std::vector<double> values(n);
    std::vector<double> vectors_flat(n * n);
    ensure(sentil_symmetric_eigen(flat.data(), n, values.data(), vectors_flat.data()));
    return {std::move(values), detail::unflatten(vectors_flat, n, n)};
}

/// Find an input sequence for the model that best satisfies the spec.
inline SynthesisResult synthesize(const SystemModel& model, const Formula& spec,
                                  const Bounds* bounds = nullptr,
                                  const SmoothConfig* smooth = nullptr,
                                  Backend backend = Backend::Auto, std::size_t max_iters = 0,
                                  std::size_t population = 0) {
    sentil_smooth_config_t sc;
    const sentil_smooth_config_t* sc_ptr = nullptr;
    if (smooth) {
        sc = detail::to_c(*smooth);
        sc_ptr = &sc;
    }
    sentil_synthesis_result_t out;
    ensure(sentil_synthesize(model.get(), spec.get(), bounds ? bounds->get() : nullptr, sc_ptr,
                             max_iters, static_cast<sentil_backend_t>(backend), population, &out));
    std::vector<double> input(out.input, out.input + out.input_len);
    sentil_free_doubles(out.input, out.input_len);
    return SynthesisResult{std::move(input), out.robustness, out.holds,
                           static_cast<Backend>(out.backend)};
}

}  // namespace synthesis

/// A least-restrictive safety shield.
class SafetyFilter {
public:
    /// A filter enforcing the given bounds.
    explicit SafetyFilter(Bounds bounds)
        : handle_(detail::must(sentil_safety_filter_create(bounds.release()))) {}

    /// The input closest to nominal that satisfies each barrier and the bounds.
    std::vector<double> filter(
        const std::vector<double>& nominal,
        const std::vector<std::pair<std::vector<double>, double>>& barriers = {}) const {
        std::size_t n = nominal.size();
        std::size_t m = barriers.size();
        std::vector<double> barrier_a;
        std::vector<double> barrier_b;
        barrier_a.reserve(m * n);
        barrier_b.reserve(m);
        for (const auto& barrier : barriers) {
            if (barrier.first.size() != n) {
                detail::raise_with(SENTIL_ERR_INVALID_CONFIG,
                                   "each barrier coefficient vector must match the input length");
            }
            barrier_a.insert(barrier_a.end(), barrier.first.begin(), barrier.first.end());
            barrier_b.push_back(barrier.second);
        }
        std::vector<double> out(n);
        ensure(sentil_safety_filter_filter(get(), nominal.data(), n, barrier_a.data(),
                                           barrier_b.data(), m, out.data()));
        return out;
    }

    explicit SafetyFilter(sentil_safety_filter_t* handle) : handle_(handle) {}

    sentil_safety_filter_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_safety_filter_t, sentil_safety_filter_destroy> handle_;
};

/// A requirement that a spec holds with at least a target probability.
class ChanceConstraint {
public:
    /// The constraint that spec holds with probability at least probability.
    ChanceConstraint(Formula spec, double probability, double confidence = 0.0,
                     double tightening = 0.0)
        : handle_(detail::must(sentil_chance_constraint_create(spec.release(), probability,
                                                               confidence, tightening))) {}

    /// Estimate satisfaction over the given number of sampled trajectories.
    ChanceReport validate(const StochasticSystem& system, std::uint64_t samples = 1000,
                          std::uint64_t seed = 42) const {
        sentil_chance_report_t out;
        ensure(sentil_chance_constraint_validate(get(), system.get(), samples, seed, &out));
        return detail::from_c(out);
    }

    explicit ChanceConstraint(sentil_chance_constraint_t* handle) : handle_(handle) {}

    sentil_chance_constraint_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_chance_constraint_t, sentil_chance_constraint_destroy> handle_;
};

/// A receding-horizon controller that emits a control input within a hard deadline.
class Controller {
public:
    /// A controller over the model and spec. budget_ns is in nanoseconds.
    Controller(SystemModel model, Formula spec, std::size_t input_width, std::uint64_t budget_ns,
               const Bounds* bounds = nullptr, const SmoothConfig* smooth = nullptr)
        : input_width_(input_width),
          handle_(make(std::move(model), std::move(spec), input_width, budget_ns, bounds, smooth)) {}

    /// Plan from the current state and return the first control input.
    std::vector<double> control(const std::vector<double>& state) {
        std::vector<double> out(input_width_);
        ensure(sentil_controller_control(get(), state.data(), state.size(), out.data()));
        return out;
    }

    Controller(sentil_controller_t* handle, std::size_t input_width)
        : input_width_(input_width), handle_(handle) {}

    sentil_controller_t* get() const { return handle_.get(); }

private:
    static sentil_controller_t* make(SystemModel model, Formula spec, std::size_t input_width,
                                     std::uint64_t budget_ns, const Bounds* bounds,
                                     const SmoothConfig* smooth) {
        sentil_smooth_config_t sc;
        const sentil_smooth_config_t* sc_ptr = nullptr;
        if (smooth) {
            sc = detail::to_c(*smooth);
            sc_ptr = &sc;
        }
        return detail::must(sentil_controller_create(model.release(), spec.release(), input_width,
                                                     budget_ns, bounds ? bounds->get() : nullptr,
                                                     sc_ptr));
    }

    std::size_t input_width_;
    detail::Handle<sentil_controller_t, sentil_controller_destroy> handle_;
};

/// A witnessing trajectory found by the counterexample search.
struct Witness {
    std::vector<double> input;
    double robustness;
    Trace trace;
};

namespace detail {

inline Witness pack_witness(sentil_witness_t& w) {
    std::vector<double> input(w.input, w.input + w.input_len);
    sentil_free_doubles(w.input, w.input_len);
    return Witness{std::move(input), w.robustness, Trace(w.trace)};
}

}  // namespace detail

inline RareEventResult Formula::check_rare_event(const StochasticSystem& system,
                                                 const RareEventConfig& config) const {
    sentil_rare_event_config_t c = detail::to_c(config);
    sentil_rare_event_result_t out;
    ensure(sentil_formula_check_rare_event(get(), system.get(), &c, &out));
    return detail::from_c(out);
}

inline RareEventResult Monitor::check_rare(const StochasticSystem& system) const {
    sentil_rare_event_result_t out;
    ensure(sentil_monitor_check_rare(get(), system.get(), &out));
    return detail::from_c(out);
}

inline Witness Formula::find_counterexample(const SystemModel& model, const Bounds& bounds,
                                            std::size_t max_iters, const SmoothConfig* smooth) const {
    sentil_smooth_config_t sc;
    const sentil_smooth_config_t* sc_ptr = nullptr;
    if (smooth) {
        sc = detail::to_c(*smooth);
        sc_ptr = &sc;
    }
    sentil_witness_t out;
    ensure(sentil_formula_find_counterexample(get(), model.get(), bounds.get(), max_iters, sc_ptr,
                                              &out));
    return detail::pack_witness(out);
}

inline Witness Formula::falsify(const SystemModel& model, const Bounds& bounds,
                                const CmaConfig& config, std::size_t restarts) const {
    sentil_cma_config_t c = detail::to_c(config);
    sentil_witness_t out;
    ensure(sentil_formula_falsify(get(), model.get(), bounds.get(), c, restarts, &out));
    return detail::pack_witness(out);
}

/// The specifications-library loader.
class SpecBuilder {
public:
    /// A builder for the named spec from the embedded registry.
    explicit SpecBuilder(const std::string& name)
        : handle_(detail::must(sentil_spec_builder_create(name.c_str()))) {}

    /// The names of every embedded specification, sorted.
    static std::vector<std::string> available() {
        std::size_t count = 0;
        char** raw = sentil_spec_registry_available(&count);
        return detail::owned_string_array(raw, count);
    }

    /// A builder loaded from a spec template file.
    static SpecBuilder from_file(const std::string& path) {
        return SpecBuilder(detail::must(sentil_spec_builder_from_file(path.c_str())));
    }

    /// Select a named variant.
    SpecBuilder with_variant(const std::string& variant) && {
        return SpecBuilder(
            detail::must(sentil_spec_builder_with_variant(release(), variant.c_str())));
    }

    /// Override a parameter.
    SpecBuilder with_param(const std::string& name, double value) && {
        return SpecBuilder(
            detail::must(sentil_spec_builder_with_param(release(), name.c_str(), value)));
    }

    /// The variant names the spec offers, sorted.
    std::vector<std::string> available_variants() const {
        std::size_t count = 0;
        char** raw = sentil_spec_builder_available_variants(get(), &count);
        return detail::owned_string_array(raw, count);
    }

    /// The deterministic formula text with the parameters filled in.
    std::string build_deterministic() const {
        return detail::owned_string(sentil_spec_builder_build_deterministic(get()));
    }

    /// The probabilistic formula text with the parameters filled in.
    std::string build_probabilistic() const {
        return detail::owned_string(sentil_spec_builder_build_probabilistic(get()));
    }

    /// The deterministic formula as a handle.
    Formula build_formula() const {
        return Formula(detail::must(sentil_spec_builder_build_formula(get())));
    }

    /// The probabilistic formula as a handle.
    Formula build_probabilistic_formula() const {
        return Formula(detail::must(sentil_spec_builder_build_probabilistic_formula(get())));
    }

    /// A lifting registry built from the spec's resolved noise models.
    LiftingRegistry build_lifting_registry() const {
        return LiftingRegistry(detail::must(sentil_spec_builder_build_lifting_registry(get())));
    }

    /// The resolved parameters as a JSON object.
    std::string parameters_json() const {
        return detail::owned_string(sentil_spec_builder_parameters_json(get()));
    }

    /// A monitor preloaded with the spec's recommended settings.
    Monitor build_monitor() && {
        return Monitor(detail::must(sentil_spec_builder_into_monitor(release())));
    }

    explicit SpecBuilder(sentil_spec_builder_t* handle) : handle_(handle) {}

    sentil_spec_builder_t* get() const { return handle_.get(); }

    sentil_spec_builder_t* release() { return handle_.release(); }

private:
    detail::Handle<sentil_spec_builder_t, sentil_spec_builder_destroy> handle_;
};

inline GpuSplittingEstimate Formula::check_rare_event_gpu(const SimModel& model,
                                                          const RareEventConfig& config) const {
    sentil_rare_event_config_t c = detail::to_c(config);
    sentil_gpu_splitting_estimate_t out;
    ensure(sentil_formula_check_rare_event_gpu(get(), model.get(), &c, &out));
    return detail::from_c(out);
}

/// The GPU accelerated paths.
namespace gpu {

/// Whether a usable GPU device is present.
inline bool is_available() { return sentil_gpu_is_available(); }

}  // namespace gpu

}  // namespace sentil

#endif  // SENTIL_HPP