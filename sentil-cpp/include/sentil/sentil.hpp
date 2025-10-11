#ifndef SENTIL_HPP
#define SENTIL_HPP

#include <sentil.h>

#include <cstddef>
#include <cstdint>
#include <map>
#include <optional>
#include <string>
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
        check(sentil_trace_add_signal(get(), name.c_str(), values.data(), values.size()));
    }

    /// Add or replace several named signals at once.
    void add_signals(const std::map<std::string, std::vector<double>>& signals) {
        for (const auto& entry : signals) {
            add_signal(entry.first, entry.second);
        }
    }

    explicit Trace(sentil_trace_t* handle) : handle_(handle) {}

    sentil_trace_t* get() const { return handle_.get(); }

    sentil_trace_t* release() { return handle_.release(); }

private:
    detail::Handle<sentil_trace_t, sentil_trace_destroy> handle_;
};

}  // namespace sentil

#endif  // SENTIL_HPP