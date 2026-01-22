#include <fstream>
#include <iostream>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <vector>

#include "cyber/cyber.h"
#include "cyber/message/protobuf_factory.h"
#include "cyber/record/record_reader.h"
#include "gflags/gflags.h"
#include "google/protobuf/text_format.h"

#include "sentil/sentil.hpp"

#include "modules/sentil/common/engine_config.h"
#include "modules/sentil/common/field_extractor.h"
#include "modules/sentil/proto/sentil_config.pb.h"

DEFINE_string(config, "", "the SentilConfig text proto");
DEFINE_string(record, "", "the cyber .record file to analyze");

namespace apollo {
namespace sentil {

::sentil::Trace ReadTrace(const std::string& record_file, const SentilConfig& config) {
  FieldExtractor extractor;
  std::vector<std::string> variables;
  for (const ChannelMapping& channel : config.input_channels()) {
    std::vector<FieldMapping> fields(channel.fields().begin(), channel.fields().end());
    extractor.add_channel(channel.message_type(), fields);
    for (const FieldMapping& field : channel.fields()) {
      variables.push_back(field.variable());
    }
  }

  apollo::cyber::record::RecordReader reader(record_file);
  apollo::cyber::record::RecordMessage message;
  auto* factory = apollo::cyber::message::ProtobufFactory::Instance();

  const std::set<std::string> wanted(variables.begin(), variables.end());
  std::map<std::string, double> state;
  std::vector<double> times;
  std::map<std::string, std::vector<double>> series;
  double last_time = -1.0;
  while (reader.ReadMessage(&message)) {
    const std::string type = reader.GetHeader().message_type(message.channel_name);
    if (type.empty()) {
      continue;
    }
    std::unique_ptr<google::protobuf::Message> proto(factory->GenerateMessageByType(type));
    if (proto == nullptr || !proto->ParseFromString(message.content)) {
      continue;
    }
    std::vector<std::string> names;
    std::vector<double> values;
    extractor.extract_into(type, *proto, &names, &values);
    for (std::size_t i = 0; i < names.size(); ++i) {
      state[names[i]] = values[i];
    }
    if (state.size() < wanted.size()) {
      continue;
    }
    const double time = static_cast<double>(message.time) / 1e9;
    if (time <= last_time) {
      continue;
    }
    last_time = time;
    times.push_back(time);
    for (const std::string& variable : variables) {
      series[variable].push_back(state[variable]);
    }
  }

  ::sentil::Trace trace(times);
  for (const std::string& variable : variables) {
    trace.add_signal(variable, series[variable]);
  }
  return trace;
}

void Analyze(const SentilConfig& config, const ::sentil::Trace& trace) {
  ::sentil::LiftingRegistry lifting = lifting_from_proto(config);
  ::sentil::SmcConfig smc = smc_from_proto(config);
  for (const Formula& formula : config.formulas()) {
    ::sentil::Formula phi = ::sentil::Formula::parse(formula.expression());
    std::cout << "formula " << formula.id() << ": " << formula.expression() << "\n";
    switch (config.algorithm()) {
      case DETERMINISTIC: {
        const double robustness =
            config.semantics() == DENSE ? phi.robustness_dense(trace) : phi.robustness(trace);
        std::cout << "  robustness " << robustness << " ("
                  << (robustness >= 0.0 ? "satisfied" : "violated") << ")\n";
        break;
      }
      case SMC: {
        const ::sentil::SmcResult result = phi.check(trace, lifting, smc);
        std::cout << "  probability " << result.probability << ", " << smc.confidence
                  << " interval [" << result.interval.lower << ", " << result.interval.upper
                  << "]\n";
        break;
      }
      case SPRT: {
        ::sentil::SprtConfig sprt;
        sprt.p0 = config.has_sprt_config() ? config.sprt_config().p0() : 0.90;
        sprt.p1 = config.has_sprt_config() ? config.sprt_config().p1() : 0.95;
        sprt.alpha = config.has_sprt_config() ? config.sprt_config().alpha() : 0.05;
        sprt.beta = config.has_sprt_config() ? config.sprt_config().beta() : 0.05;
        sprt.max_samples = config.sample_budget();
        const ::sentil::SprtResult result = phi.check_sequential(trace, lifting, sprt);
        std::cout << "  sprt decided in " << result.samples << " samples\n";
        break;
      }
      case AMS:
        std::cerr << "  AMS is rare-event splitting over a stochastic model, not a recorded "
                     "trace; reach for the CLI rare-event path instead\n";
        break;
    }
  }
}

}  // namespace sentil
}  // namespace apollo

int main(int argc, char** argv) {
  gflags::ParseCommandLineFlags(&argc, &argv, true);
  apollo::cyber::Init(argv[0]);
  if (FLAGS_config.empty() || FLAGS_record.empty()) {
    std::cerr << "usage: " << argv[0] << " --config=<pb.txt> --record=<file.record>\n";
    return 1;
  }
  apollo::sentil::SentilConfig config;
  std::ifstream in(FLAGS_config);
  const std::string text((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());
  if (!google::protobuf::TextFormat::ParseFromString(text, &config) || config.formulas().empty()) {
    std::cerr << "could not parse a SentilConfig with at least one formula\n";
    return 1;
  }
  try {
    const ::sentil::Trace trace = apollo::sentil::ReadTrace(FLAGS_record, config);
    apollo::sentil::Analyze(config, trace);
  } catch (const std::exception& error) {
    std::cerr << "analysis failed: " << error.what() << "\n";
    return 1;
  }
  return 0;
}