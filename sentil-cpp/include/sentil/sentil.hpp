#ifndef SENTIL_HPP
#define SENTIL_HPP

#include <sentil.h>

#include <cstddef>
#include <cstdint>
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

    explicit Formula(sentil_formula_t* handle) : handle_(handle) {}

    sentil_formula_t* get() const { return handle_.get(); }

private:
    detail::Handle<sentil_formula_t, sentil_formula_destroy> handle_;
};

}  // namespace sentil

#endif  // SENTIL_HPP