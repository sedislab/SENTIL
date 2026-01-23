#ifndef ARA_LOG_LOG_H
#define ARA_LOG_LOG_H

#include <iostream>
#include <string>
#include <utility>

namespace ara {
namespace log {

class LogStream {
 public:
  LogStream(const std::string& context, const std::string& level) {
    std::cerr << "[" << context << "][" << level << "] ";
  }
  ~LogStream() { std::cerr << "\n"; }

  template <typename T>
  LogStream& operator<<(T&& value) {
    std::cerr << std::forward<T>(value);
    return *this;
  }
};

class Logger {
 public:
  explicit Logger(std::string context) : context_(std::move(context)) {}
  LogStream LogInfo() const { return LogStream(context_, "info"); }
  LogStream LogWarn() const { return LogStream(context_, "warn"); }
  LogStream LogError() const { return LogStream(context_, "error"); }

 private:
  std::string context_;
};

inline Logger CreateLogger(const std::string& context) { return Logger(context); }

}  // namespace log
}  // namespace ara

#endif  // ARA_LOG_LOG_H