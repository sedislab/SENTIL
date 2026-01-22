#ifndef ARA_CORE_CORE_H
#define ARA_CORE_CORE_H

// A focused open-source subset of ara::core: the error, result, and future types the
// SENTIL apps use. A vendor AP swaps this header out for its own ara::core through the
// toolchain; the app source is unchanged.
#include <future>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ara {
namespace core {

using String = std::string;

template <typename T>
using Vector = std::vector<T>;

/// A domain-tagged error code with a human-readable message.
class ErrorCode {
 public:
  ErrorCode() = default;
  explicit ErrorCode(std::string message) : message_(std::move(message)), failed_(true) {}
  bool failed() const { return failed_; }
  const std::string& Message() const { return message_; }

 private:
  std::string message_;
  bool failed_ = false;
};

/// A value or an error, never both. The SENTIL surface throws SentilError, which the apps
/// catch at the boundary and turn into a failed Result.
template <typename T>
class Result {
 public:
  Result(T value) : value_(std::move(value)) {}              // NOLINT(runtime/explicit)
  Result(ErrorCode error) : error_(std::move(error)) {}      // NOLINT(runtime/explicit)

  bool HasValue() const { return !error_.failed(); }
  explicit operator bool() const { return HasValue(); }
  const T& Value() const { return value_; }
  T& Value() { return value_; }
  const ErrorCode& Error() const { return error_; }

  static Result FromValue(T value) { return Result(std::move(value)); }
  static Result FromError(std::string message) { return Result(ErrorCode(std::move(message))); }

 private:
  T value_{};
  ErrorCode error_;
};

template <typename T>
using Future = std::future<T>;

}  // namespace core
}  // namespace ara

#endif  // ARA_CORE_CORE_H