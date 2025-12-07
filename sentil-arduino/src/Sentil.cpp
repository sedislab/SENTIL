#include "Sentil.h"

SentilMonitor::SentilMonitor() : handle_(nullptr) {}

SentilMonitor::~SentilMonitor() { sentil_embedded_destroy(handle_); }

sentil_embedded_status_t SentilMonitor::begin(const char *formula) {
    sentil_embedded_destroy(handle_);
    handle_ = nullptr;
    return sentil_embedded_create(formula, &handle_);
}

sentil_embedded_status_t SentilMonitor::beginCompiled(const uint8_t *bytes, size_t len) {
    sentil_embedded_destroy(handle_);
    handle_ = nullptr;
    return sentil_embedded_create_compiled(bytes, len, &handle_);
}

bool SentilMonitor::ready() const { return handle_ != nullptr; }

sentil_embedded_status_t SentilMonitor::update(double time, const double *values, size_t n,
                                               sentil_embedded_robustness_t &out) {
    return sentil_embedded_update(handle_, time, values, n, &out);
}

size_t SentilMonitor::variableCount() const { return sentil_embedded_variable_count(handle_); }

bool SentilMonitor::symbolIndex(const char *name, size_t &index) const {
    bool found = false;
    sentil_embedded_symbol_index(handle_, name, &index, &found);
    return found;
}

void SentilMonitor::reset() { sentil_embedded_reset(handle_); }