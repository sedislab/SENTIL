#ifndef ARA_EXEC_EXECUTION_CLIENT_H
#define ARA_EXEC_EXECUTION_CLIENT_H

#include <cstdio>

namespace ara {
namespace exec {

enum class ExecutionState { kRunning };

class ExecutionClient {
 public:
  void ReportExecutionState(ExecutionState state) {
    if (state == ExecutionState::kRunning) {
      std::fprintf(stderr, "[ara::exec] execution state Running\n");
    }
  }
};

}  // namespace exec
}  // namespace ara

#endif  // ARA_EXEC_EXECUTION_CLIENT_H