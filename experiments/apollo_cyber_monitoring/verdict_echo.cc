#include <cstdio>
#include <memory>

#include "cyber/cyber.h"

#include "modules/sentil/proto/sentil_status.pb.h"

using apollo::sentil::SentilStatus;

int main(int argc, char** argv) {
  apollo::cyber::Init(argv[0]);
  auto node = apollo::cyber::CreateNode("sentil_verdict_echo");
  auto reader = node->CreateReader<SentilStatus>(
      "/apollo/sentil/status", [](const std::shared_ptr<SentilStatus>& s) {
        for (const auto& r : s->results()) {
          std::printf("t=%.2f formula %lu satisfied=%d", s->header().timestamp_sec(),
                      static_cast<unsigned long>(r.id()), r.satisfied() ? 1 : 0);
          if (r.has_prob_result()) {
            std::printf(" P=%.4f [%.4f, %.4f]", r.prob_result().probability(),
                        r.prob_result().interval().lower(), r.prob_result().interval().upper());
          } else if (r.has_robustness()) {
            std::printf(" robustness=%.4f concrete=%d", r.robustness().min(),
                        r.robustness().is_concrete() ? 1 : 0);
          }
          std::printf("\n");
        }
        std::fflush(stdout);
      });
  (void)reader;
  apollo::cyber::WaitForShutdown();
  return 0;
}