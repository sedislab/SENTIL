#ifndef SENTIL_HPP
#define SENTIL_HPP

#include <sentil.h>

#include <cstdint>

#include "sentil/errors.hpp"

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

}  // namespace sentil

#endif  // SENTIL_HPP