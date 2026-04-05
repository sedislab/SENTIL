#ifndef SENTIL_VENDOR_CONTRACT_ARA_LOG_LOG_H
#define SENTIL_VENDOR_CONTRACT_ARA_LOG_LOG_H

// The ara::log slice the applications use, declared for the vendor_contract_check compile.
#include <string>

namespace ara {
namespace log {

class LogStream {
 public:
  LogStream(const std::string& context, const std::string& level);
  ~LogStream();

  template <typename T>
  LogStream& operator<<(T&& value);
};

class Logger {
 public:
  explicit Logger(std::string context);

  LogStream LogInfo() const;
  LogStream LogWarn() const;
  LogStream LogError() const;
};

Logger CreateLogger(const std::string& context);

}  // namespace log
}  // namespace ara

#endif  // SENTIL_VENDOR_CONTRACT_ARA_LOG_LOG_H