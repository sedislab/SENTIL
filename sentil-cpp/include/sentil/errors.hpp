// The exception hierarchy and the helpers. It follows the house style of catching 
// SentilError for everything, or a subclass to separate the failure kinds.
#ifndef SENTIL_ERRORS_HPP
#define SENTIL_ERRORS_HPP

#include <sentil.h>

#include <exception>
#include <string>
#include <utility>

namespace sentil {

/// Base class for every error SENTIL raises.
class SentilError : public std::exception {
public:
    SentilError(sentil_error_t code, std::string message)
        : code_(code), message_(std::move(message)) {}

    /// The C ABI status code behind this error.
    sentil_error_t code() const noexcept { return code_; }

    const char* what() const noexcept override { return message_.c_str(); }

private:
    sentil_error_t code_;
    std::string message_;
};

/// A formula failed to parse.
class ParseError : public SentilError {
public:
    using SentilError::SentilError;
};

/// A well-formed formula means something invalid.
class SemanticError : public SentilError {
public:
    using SentilError::SentilError;
};

/// An evaluation, data, fit, or numeric error.
class EvaluationError : public SentilError {
public:
    using SentilError::SentilError;
};

namespace detail {

/// The C ABI sizes the message on a first call with a null buffer.
inline std::string last_error_message() {
    std::size_t needed = sentil_get_last_error_message(nullptr, 0);
    if (needed <= 1) {
        return std::string();
    }
    std::string buffer(needed - 1, '\0');
    sentil_get_last_error_message(&buffer[0], needed);
    return buffer;
}

[[noreturn]] inline void raise_with(sentil_error_t code, std::string message) {
    switch (code) {
        case SENTIL_ERR_PARSE:
            throw ParseError(code, std::move(message));
        case SENTIL_ERR_UNKNOWN_VARIABLE:
        case SENTIL_ERR_NOT_PROBABILISTIC:
        case SENTIL_ERR_UNSUPPORTED:
            throw SemanticError(code, std::move(message));
        default:
            throw EvaluationError(code, std::move(message));
    }
}

[[noreturn]] inline void raise(sentil_error_t code) {
    std::string message = last_error_message();
    if (message.empty()) {
        message = "SENTIL error";
    }
    raise_with(code, std::move(message));
}

[[noreturn]] inline void raise_last() {
    raise(sentil_get_last_error_code());
}

}  // namespace detail

/// Throw the matching error subclass when code is not SENTIL_OK.
inline void check(sentil_error_t code) {
    if (code != SENTIL_OK) {
        detail::raise(code);
    }
}

}  // namespace sentil

#endif  // SENTIL_ERRORS_HPP