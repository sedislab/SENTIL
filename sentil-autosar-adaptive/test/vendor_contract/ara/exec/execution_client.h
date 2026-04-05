#ifndef SENTIL_VENDOR_CONTRACT_ARA_EXEC_EXECUTION_CLIENT_H
#define SENTIL_VENDOR_CONTRACT_ARA_EXEC_EXECUTION_CLIENT_H

// The ara::exec slice the applications use, declared for the vendor_contract_check compile.
namespace ara {
namespace exec {

enum class ExecutionState { kRunning };

class ExecutionClient {
 public:
  void ReportExecutionState(ExecutionState state);
};

}  // namespace exec
}  // namespace ara

#endif  // SENTIL_VENDOR_CONTRACT_ARA_EXEC_EXECUTION_CLIENT_H