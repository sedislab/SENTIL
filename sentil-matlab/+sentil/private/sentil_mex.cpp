/* Command-dispatch gateway for the +sentil package. */

#include "mex.h"
#include "sentil.h"

#include <cstdint>
#include <string>
#include <vector>

namespace {

const char* error_id(sentil_error_t code) {
    switch (code) {
        case SENTIL_ERR_PARSE:
            return "sentil:parse";
        case SENTIL_ERR_UNKNOWN_VARIABLE:
        case SENTIL_ERR_NOT_PROBABILISTIC:
        case SENTIL_ERR_UNSUPPORTED:
            return "sentil:semantic";
        default:
            return "sentil:evaluation";
    }
}

void raise_last(sentil_error_t code) {
    size_t needed = sentil_get_last_error_message(nullptr, 0);
    std::string message;
    if (needed > 0) {
        message.resize(needed);
        sentil_get_last_error_message(&message[0], needed);
        message.resize(needed - 1);
    }
    mexErrMsgIdAndTxt(error_id(code), "%s", message.empty() ? "sentil error" : message.c_str());
}

void throw_sentil(sentil_error_t code, const std::string& message) {
    mexErrMsgIdAndTxt(error_id(code), "%s", message.c_str());
}

void check(sentil_error_t code) {
    if (code != SENTIL_OK) {
        raise_last(code);
    }
}

/* A NULL handle from a fallible constructor carries its reason in the last-error slot. */
void* checked(void* handle) {
    if (handle == nullptr) {
        raise_last(sentil_get_last_error_code());
    }
    return handle;
}

// mexErrMsgIdAndTxt longjmps back to MATLAB; control never returns past this.
void fail(const std::string& message) {
    mexErrMsgIdAndTxt("sentil:mex", "%s", message.c_str());
}

void need(int nrhs, int count) {
    if (nrhs < count) {
        fail("a command was called with too few arguments");
    }
}

std::string get_string(const mxArray* arr) {
    if (arr == nullptr || !mxIsChar(arr)) {
        fail("expected a string argument");
    }
    char* raw = mxArrayToUTF8String(arr);
    if (raw == nullptr) {
        fail("could not read a string argument");
    }
    std::string value(raw);
    mxFree(raw);
    return value;
}

template <typename T>
T* get_handle(const mxArray* arr) {
    if (arr == nullptr || !mxIsUint64(arr) || mxGetNumberOfElements(arr) != 1) {
        fail("expected a handle");
    }
    return reinterpret_cast<T*>(*static_cast<uint64_t*>(mxGetData(arr)));
}

mxArray* make_handle(void* ptr) {
    mxArray* arr = mxCreateNumericMatrix(1, 1, mxUINT64_CLASS, mxREAL);
    *static_cast<uint64_t*>(mxGetData(arr)) = reinterpret_cast<uint64_t>(ptr);
    return arr;
}

/* mxCreateString assumes ASCII, and engine text can be non-ASCII. */
mxArray* make_string(const char* utf8) {
    if (utf8 == nullptr) {
        return mxCreateString("");
    }
    std::vector<mxChar> units;
    const unsigned char* p = reinterpret_cast<const unsigned char*>(utf8);
    while (*p != 0) {
        uint32_t cp;
        unsigned char c = *p;
        if (c < 0x80) {
            cp = c;
            p += 1;
        } else if ((c >> 5) == 0x6) {
            cp = ((c & 0x1Fu) << 6) | (p[1] & 0x3Fu);
            p += 2;
        } else if ((c >> 4) == 0xE) {
            cp = ((c & 0x0Fu) << 12) | ((p[1] & 0x3Fu) << 6) | (p[2] & 0x3Fu);
            p += 3;
        } else if ((c >> 3) == 0x1E) {
            cp = ((c & 0x07u) << 18) | ((p[1] & 0x3Fu) << 12) | ((p[2] & 0x3Fu) << 6) |
                 (p[3] & 0x3Fu);
            p += 4;
        } else {
            cp = 0xFFFD;
            p += 1;
        }
        if (cp <= 0xFFFF) {
            units.push_back(static_cast<mxChar>(cp));
        } else {
            cp -= 0x10000;
            units.push_back(static_cast<mxChar>(0xD800 + (cp >> 10)));
            units.push_back(static_cast<mxChar>(0xDC00 + (cp & 0x3FF)));
        }
    }
    mwSize dims[2] = {1, static_cast<mwSize>(units.size())};
    mxArray* arr = mxCreateCharArray(2, dims);
    mxChar* out = mxGetChars(arr);
    for (size_t i = 0; i < units.size(); ++i) {
        out[i] = units[i];
    }
    return arr;
}

mxArray* make_string_array(char** strings, size_t count) {
    mxArray* cell = mxCreateCellMatrix(1, static_cast<mwSize>(count));
    for (size_t i = 0; i < count; ++i) {
        mxSetCell(cell, static_cast<mwIndex>(i), make_string(strings[i]));
    }
    return cell;
}

const double* get_doubles(const mxArray* arr, size_t* count) {
    if (arr == nullptr || !mxIsDouble(arr) || mxIsComplex(arr)) {
        fail("expected a real double array");
    }
    *count = mxGetNumberOfElements(arr);
    return mxGetDoubles(arr);
}

double get_scalar(const mxArray* arr) {
    // mxIsDouble holds for a complex array, and mxGetDoubles returns null for one,
    // so the complex test has to come before the dereference.
    if (arr == nullptr || !mxIsDouble(arr) || mxIsComplex(arr) ||
        mxGetNumberOfElements(arr) != 1) {
        fail("expected a real scalar");
    }
    return *mxGetDoubles(arr);
}

// A caller-declared dimension has to agree with the array it describes, or the
// engine reads past the end of a MATLAB-owned buffer.
void need_length(size_t actual, size_t declared, const char* what) {
    if (actual != declared) {
        fail(std::string("expected ") + std::to_string(declared) + " elements in " + what +
             " but got " + std::to_string(actual));
    }
}

mxArray* make_doubles(const double* data, size_t n) {
    mxArray* arr = mxCreateDoubleMatrix(1, static_cast<mwSize>(n), mxREAL);
    double* out = mxGetDoubles(arr);
    for (size_t i = 0; i < n; ++i) {
        out[i] = data[i];
    }
    return arr;
}

mxArray* make_intervals(const sentil_interval_t* spans, size_t count) {
    mxArray* arr = mxCreateDoubleMatrix(static_cast<mwSize>(count), 2, mxREAL);
    double* out = mxGetDoubles(arr);
    for (size_t i = 0; i < count; ++i) {
        out[i] = spans[i].start;
        out[count + i] = spans[i].end;
    }
    return arr;
}

const char* kRobustnessFields[] = {"resolved", "satisfied", "value", "lower", "upper"};

void set_robustness(mxArray* target, mwIndex i, const sentil_robustness_t& r) {
    mxSetField(target, i, "resolved", mxCreateLogicalScalar(r.resolved));
    mxSetField(target, i, "satisfied", mxCreateLogicalScalar(r.satisfied));
    mxSetField(target, i, "value", mxCreateDoubleScalar(r.value));
    mxSetField(target, i, "lower", mxCreateDoubleScalar(r.lower));
    mxSetField(target, i, "upper", mxCreateDoubleScalar(r.upper));
}

mxArray* make_robustness(const sentil_robustness_t& r) {
    mxArray* s = mxCreateStructMatrix(1, 1, 5, kRobustnessFields);
    set_robustness(s, 0, r);
    return s;
}

mxArray* make_robustness_array(const sentil_robustness_t* arr, size_t count) {
    mxArray* s = mxCreateStructMatrix(1, static_cast<mwSize>(count), 5, kRobustnessFields);
    for (size_t i = 0; i < count; ++i) {
        set_robustness(s, static_cast<mwIndex>(i), arr[i]);
    }
    return s;
}

mxArray* make_confidence(const sentil_confidence_interval_t& ci) {
    static const char* fields[] = {"lower", "upper", "level"};
    mxArray* s = mxCreateStructMatrix(1, 1, 3, fields);
    mxSetField(s, 0, "lower", mxCreateDoubleScalar(ci.lower));
    mxSetField(s, 0, "upper", mxCreateDoubleScalar(ci.upper));
    mxSetField(s, 0, "level", mxCreateDoubleScalar(ci.level));
    return s;
}

mxArray* make_named_robustness(const sentil_named_robustness_t* arr, size_t count) {
    static const char* fields[] = {"id", "resolved", "satisfied", "value", "lower", "upper"};
    mxArray* s = mxCreateStructMatrix(1, static_cast<mwSize>(count), 6, fields);
    for (size_t i = 0; i < count; ++i) {
        mwIndex j = static_cast<mwIndex>(i);
        mxSetField(s, j, "id", make_string(arr[i].id));
        mxSetField(s, j, "resolved", mxCreateLogicalScalar(arr[i].robustness.resolved));
        mxSetField(s, j, "satisfied", mxCreateLogicalScalar(arr[i].robustness.satisfied));
        mxSetField(s, j, "value", mxCreateDoubleScalar(arr[i].robustness.value));
        mxSetField(s, j, "lower", mxCreateDoubleScalar(arr[i].robustness.lower));
        mxSetField(s, j, "upper", mxCreateDoubleScalar(arr[i].robustness.upper));
    }
    return s;
}

mxArray* make_bank(sentil_bank_result_t* owned, size_t count) {
    for (size_t i = 0; i < count; ++i) {
        if (!owned[i].ok) {
            std::string message =
                std::string("formula '") + (owned[i].id ? owned[i].id : "") + "' failed to evaluate";
            sentil_error_t code = owned[i].code;
            sentil_free_bank_results(owned, count);
            throw_sentil(code, message);
        }
    }
    static const char* fields[] = {"ids", "values"};
    mxArray* s = mxCreateStructMatrix(1, 1, 2, fields);
    mxArray* ids = mxCreateCellMatrix(1, static_cast<mwSize>(count));
    mxArray* values = mxCreateDoubleMatrix(1, static_cast<mwSize>(count), mxREAL);
    double* out = mxGetDoubles(values);
    for (size_t i = 0; i < count; ++i) {
        mxSetCell(ids, static_cast<mwIndex>(i), make_string(owned[i].id));
        out[i] = owned[i].value;
    }
    mxSetField(s, 0, "ids", ids);
    mxSetField(s, 0, "values", values);
    sentil_free_bank_results(owned, count);
    return s;
}

/* storage is filled completely before ptrs; growing it would dangle the c_str pointers. */
void get_string_cell(const mxArray* cell, std::vector<std::string>& storage,
                     std::vector<const char*>& ptrs) {
    if (cell == nullptr || !mxIsCell(cell)) {
        fail("expected a cell array of names");
    }
    size_t n = mxGetNumberOfElements(cell);
    storage.reserve(n);
    for (size_t i = 0; i < n; ++i) {
        storage.push_back(get_string(mxGetCell(cell, static_cast<mwIndex>(i))));
    }
    ptrs.reserve(n);
    for (size_t i = 0; i < n; ++i) {
        ptrs.push_back(storage[i].c_str());
    }
}

}  // namespace

void mexFunction(int nlhs, mxArray* plhs[], int nrhs, const mxArray* prhs[]) {
    (void)nlhs;
    if (nrhs < 1 || !mxIsChar(prhs[0])) {
        fail("the first argument must be a command string");
    }
    std::string cmd = get_string(prhs[0]);

    if (cmd == "version") {
        uint32_t major = 0, minor = 0, patch = 0;
        sentil_version(&major, &minor, &patch);
        plhs[0] = mxCreateDoubleMatrix(1, 3, mxREAL);
        double* out = mxGetDoubles(plhs[0]);
        out[0] = major;
        out[1] = minor;
        out[2] = patch;
    } else if (cmd == "formula_parse") {
        need(nrhs, 2);
        std::string text = get_string(prhs[1]);
        plhs[0] = make_handle(checked(sentil_formula_parse(text.c_str())));
    } else if (cmd == "formula_destroy") {
        need(nrhs, 2);
        sentil_formula_destroy(get_handle<sentil_formula_t>(prhs[1]));
    } else if (cmd == "formula_variables") {
        need(nrhs, 2);
        size_t count = 0;
        char** names = sentil_formula_variables(get_handle<sentil_formula_t>(prhs[1]), &count);
        if (names == nullptr && sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_string_array(names, count);
        if (names != nullptr) {
            sentil_free_string_array(names, count);
        }
    } else if (cmd == "formula_to_json") {
        need(nrhs, 2);
        char* json = sentil_formula_to_json(get_handle<sentil_formula_t>(prhs[1]));
        plhs[0] = make_string(json);
        sentil_free_string(json);
    } else if (cmd == "formula_robustness") {
        need(nrhs, 3);
        double value = 0.0;
        check(sentil_formula_robustness(get_handle<sentil_formula_t>(prhs[1]),
                                        get_handle<sentil_trace_t>(prhs[2]), &value));
        plhs[0] = mxCreateDoubleScalar(value);
    } else if (cmd == "formula_robustness_dense") {
        need(nrhs, 3);
        double value = 0.0;
        check(sentil_formula_robustness_dense(get_handle<sentil_formula_t>(prhs[1]),
                                              get_handle<sentil_trace_t>(prhs[2]), &value));
        plhs[0] = mxCreateDoubleScalar(value);
    } else if (cmd == "formula_robustness_signal") {
        need(nrhs, 3);
        size_t len = 0;
        double* signal = sentil_formula_robustness_signal(
            get_handle<sentil_formula_t>(prhs[1]), get_handle<sentil_trace_t>(prhs[2]), &len);
        if (signal == nullptr) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_doubles(signal, len);
        sentil_free_doubles(signal, len);
    } else if (cmd == "formula_robustness_dense_signal") {
        need(nrhs, 3);
        size_t len = 0;
        double* signal = sentil_formula_robustness_dense_signal(
            get_handle<sentil_formula_t>(prhs[1]), get_handle<sentil_trace_t>(prhs[2]), &len);
        if (signal == nullptr) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_doubles(signal, len);
        sentil_free_doubles(signal, len);
    } else if (cmd == "formula_violations") {
        need(nrhs, 3);
        size_t count = 0;
        sentil_interval_t* spans = sentil_formula_violations(
            get_handle<sentil_formula_t>(prhs[1]), get_handle<sentil_trace_t>(prhs[2]), &count);
        if (spans == nullptr && sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_intervals(spans, count);
        if (spans != nullptr) {
            sentil_free_intervals(spans, count);
        }
    } else if (cmd == "trace_create") {
        need(nrhs, 2);
        size_t n = 0;
        const double* times = get_doubles(prhs[1], &n);
        plhs[0] = make_handle(checked(sentil_trace_create(times, n)));
    } else if (cmd == "trace_from_signal") {
        need(nrhs, 4);
        size_t n = 0, m = 0;
        const double* times = get_doubles(prhs[1], &n);
        std::string name = get_string(prhs[2]);
        const double* values = get_doubles(prhs[3], &m);
        if (n != m) {
            fail("the time and value vectors must have the same length");
        }
        plhs[0] = make_handle(checked(sentil_trace_from_signal(times, n, name.c_str(), values, m)));
    } else if (cmd == "trace_add_signal") {
        need(nrhs, 4);
        size_t m = 0;
        std::string name = get_string(prhs[2]);
        const double* values = get_doubles(prhs[3], &m);
        check(sentil_trace_add_signal(get_handle<sentil_trace_t>(prhs[1]), name.c_str(), values, m));
    } else if (cmd == "trace_from_csv") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_trace_from_csv(get_string(prhs[1]).c_str())));
    } else if (cmd == "trace_from_tsv") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_trace_from_tsv(get_string(prhs[1]).c_str())));
    } else if (cmd == "trace_from_path") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_trace_from_path(get_string(prhs[1]).c_str())));
    } else if (cmd == "trace_destroy") {
        need(nrhs, 2);
        sentil_trace_destroy(get_handle<sentil_trace_t>(prhs[1]));
    } else if (cmd == "trace_len") {
        need(nrhs, 2);
        plhs[0] = mxCreateDoubleScalar(
            static_cast<double>(sentil_trace_len(get_handle<sentil_trace_t>(prhs[1]))));
    } else if (cmd == "trace_is_empty") {
        need(nrhs, 2);
        plhs[0] = mxCreateLogicalScalar(sentil_trace_is_empty(get_handle<sentil_trace_t>(prhs[1])));
    } else if (cmd == "trace_times") {
        need(nrhs, 2);
        size_t len = 0;
        const double* times = sentil_trace_times(get_handle<sentil_trace_t>(prhs[1]), &len);
        plhs[0] = times ? make_doubles(times, len) : mxCreateDoubleMatrix(1, 0, mxREAL);
    } else if (cmd == "trace_variables") {
        need(nrhs, 2);
        size_t count = 0;
        char** names = sentil_trace_variables(get_handle<sentil_trace_t>(prhs[1]), &count);
        plhs[0] = make_string_array(names, count);
        if (names != nullptr) {
            sentil_free_string_array(names, count);
        }
    } else if (cmd == "trace_signal") {
        need(nrhs, 3);
        std::string name = get_string(prhs[2]);
        size_t len = 0;
        const double* signal =
            sentil_trace_signal(get_handle<sentil_trace_t>(prhs[1]), name.c_str(), &len);
        plhs[0] = signal ? make_doubles(signal, len) : mxCreateDoubleMatrix(1, 0, mxREAL);
    } else if (cmd == "trace_resample") {
        need(nrhs, 4);
        size_t n = 0;
        const double* times = get_doubles(prhs[2], &n);
        int interp = static_cast<int>(get_scalar(prhs[3]));
        plhs[0] = make_handle(checked(sentil_trace_resample(
            get_handle<sentil_trace_t>(prhs[1]), times, n,
            static_cast<sentil_interpolation_t>(interp))));
    } else if (cmd == "monitor_config_create") {
        plhs[0] = make_handle(checked(sentil_monitor_config_create()));
    } else if (cmd == "monitor_config_set_time") {
        need(nrhs, 3);
        check(sentil_monitor_config_set_time(
            get_handle<sentil_monitor_config_t>(prhs[1]),
            static_cast<sentil_time_mode_t>(static_cast<int>(get_scalar(prhs[2])))));
    } else if (cmd == "monitor_config_time_mode") {
        need(nrhs, 2);
        plhs[0] = mxCreateDoubleScalar(static_cast<double>(
            sentil_monitor_config_time_mode(get_handle<sentil_monitor_config_t>(prhs[1]))));
    } else if (cmd == "monitor_config_destroy") {
        need(nrhs, 2);
        sentil_monitor_config_destroy(get_handle<sentil_monitor_config_t>(prhs[1]));
    } else if (cmd == "monitor_create") {
        need(nrhs, 3);
        plhs[0] = make_handle(checked(sentil_monitor_create(
            get_handle<sentil_formula_t>(prhs[1]), get_handle<sentil_monitor_config_t>(prhs[2]))));
    } else if (cmd == "monitor_parse") {
        need(nrhs, 3);
        std::string text = get_string(prhs[1]);
        plhs[0] = make_handle(checked(
            sentil_monitor_parse(text.c_str(), get_handle<sentil_monitor_config_t>(prhs[2]))));
    } else if (cmd == "monitor_destroy") {
        need(nrhs, 2);
        sentil_monitor_destroy(get_handle<sentil_monitor_t>(prhs[1]));
    } else if (cmd == "monitor_robustness") {
        need(nrhs, 3);
        double value = 0.0;
        check(sentil_monitor_robustness(get_handle<sentil_monitor_t>(prhs[1]),
                                        get_handle<sentil_trace_t>(prhs[2]), &value));
        plhs[0] = mxCreateDoubleScalar(value);
    } else if (cmd == "monitor_robustness_signal") {
        need(nrhs, 3);
        size_t len = 0;
        double* signal = sentil_monitor_robustness_signal(
            get_handle<sentil_monitor_t>(prhs[1]), get_handle<sentil_trace_t>(prhs[2]), &len);
        if (signal == nullptr) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_doubles(signal, len);
        sentil_free_doubles(signal, len);
    } else if (cmd == "monitor_violations") {
        need(nrhs, 3);
        size_t count = 0;
        sentil_interval_t* spans = sentil_monitor_violations(
            get_handle<sentil_monitor_t>(prhs[1]), get_handle<sentil_trace_t>(prhs[2]), &count);
        if (spans == nullptr && sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_intervals(spans, count);
        if (spans != nullptr) {
            sentil_free_intervals(spans, count);
        }
    } else if (cmd == "monitor_symbol_index") {
        need(nrhs, 3);
        std::string name = get_string(prhs[2]);
        size_t index = 0;
        bool found = false;
        check(sentil_monitor_symbol_index(get_handle<sentil_monitor_t>(prhs[1]), name.c_str(),
                                          &index, &found));
        plhs[0] = found ? mxCreateDoubleScalar(static_cast<double>(index + 1))
                        : mxCreateDoubleMatrix(1, 0, mxREAL);
    } else if (cmd == "monitor_update") {
        need(nrhs, 5);
        sentil_monitor_t* monitor = get_handle<sentil_monitor_t>(prhs[1]);
        double time = get_scalar(prhs[2]);
        std::vector<std::string> storage;
        std::vector<const char*> names;
        get_string_cell(prhs[3], storage, names);
        size_t m = 0;
        const double* values = get_doubles(prhs[4], &m);
        if (m != names.size()) {
            fail("the names and values must have the same length");
        }
        sentil_robustness_t out;
        check(sentil_monitor_update(monitor, time, names.data(), values, m, &out));
        plhs[0] = make_robustness(out);
    } else if (cmd == "monitor_update_packed") {
        need(nrhs, 4);
        size_t m = 0;
        double time = get_scalar(prhs[2]);
        const double* values = get_doubles(prhs[3], &m);
        sentil_robustness_t out;
        check(sentil_monitor_update_packed(get_handle<sentil_monitor_t>(prhs[1]), time, values, m,
                                           &out));
        plhs[0] = make_robustness(out);
    } else if (cmd == "monitor_reset") {
        need(nrhs, 2);
        sentil_monitor_reset(get_handle<sentil_monitor_t>(prhs[1]));
    } else if (cmd == "monitor_formula") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_monitor_formula(get_handle<sentil_monitor_t>(prhs[1]))));
    } else if (cmd == "monitor_config_of") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_monitor_config(get_handle<sentil_monitor_t>(prhs[1]))));
    } else if (cmd == "stream_monitor_create") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_stream_monitor_create(get_string(prhs[1]).c_str())));
    } else if (cmd == "stream_monitor_from_formula") {
        need(nrhs, 2);
        plhs[0] = make_handle(
            checked(sentil_stream_monitor_from_formula(get_handle<sentil_formula_t>(prhs[1]))));
    } else if (cmd == "stream_monitor_variable_count") {
        need(nrhs, 2);
        plhs[0] = mxCreateDoubleScalar(static_cast<double>(
            sentil_stream_monitor_variable_count(get_handle<sentil_stream_monitor_t>(prhs[1]))));
    } else if (cmd == "stream_monitor_symbol_index") {
        need(nrhs, 3);
        std::string name = get_string(prhs[2]);
        size_t index = 0;
        bool found = false;
        check(sentil_stream_monitor_symbol_index(get_handle<sentil_stream_monitor_t>(prhs[1]),
                                                 name.c_str(), &index, &found));
        plhs[0] = found ? mxCreateDoubleScalar(static_cast<double>(index + 1))
                        : mxCreateDoubleMatrix(1, 0, mxREAL);
    } else if (cmd == "stream_monitor_update") {
        need(nrhs, 5);
        sentil_stream_monitor_t* monitor = get_handle<sentil_stream_monitor_t>(prhs[1]);
        double time = get_scalar(prhs[2]);
        std::vector<std::string> storage;
        std::vector<const char*> names;
        get_string_cell(prhs[3], storage, names);
        size_t m = 0;
        const double* values = get_doubles(prhs[4], &m);
        if (m != names.size()) {
            fail("the names and values must have the same length");
        }
        sentil_robustness_t out;
        check(sentil_stream_monitor_update(monitor, time, names.data(), values, m, &out));
        plhs[0] = make_robustness(out);
    } else if (cmd == "stream_monitor_update_packed") {
        need(nrhs, 4);
        size_t m = 0;
        double time = get_scalar(prhs[2]);
        const double* values = get_doubles(prhs[3], &m);
        sentil_robustness_t out;
        check(sentil_stream_monitor_update_packed(get_handle<sentil_stream_monitor_t>(prhs[1]), time,
                                                  values, m, &out));
        plhs[0] = make_robustness(out);
    } else if (cmd == "stream_monitor_run") {
        need(nrhs, 3);
        size_t count = 0;
        sentil_robustness_t* run = sentil_stream_monitor_run(
            get_handle<sentil_stream_monitor_t>(prhs[1]), get_handle<sentil_trace_t>(prhs[2]),
            &count);
        if (run == nullptr && sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_robustness_array(run, count);
        if (run != nullptr) {
            sentil_free_robustness(run, count);
        }
    } else if (cmd == "stream_monitor_reset") {
        need(nrhs, 2);
        sentil_stream_monitor_reset(get_handle<sentil_stream_monitor_t>(prhs[1]));
    } else if (cmd == "stream_monitor_destroy") {
        need(nrhs, 2);
        sentil_stream_monitor_destroy(get_handle<sentil_stream_monitor_t>(prhs[1]));
    } else if (cmd == "multi_monitor_create") {
        plhs[0] = make_handle(checked(sentil_multi_monitor_create()));
    } else if (cmd == "multi_monitor_add") {
        need(nrhs, 4);
        std::string id = get_string(prhs[2]);
        std::string formula = get_string(prhs[3]);
        check(sentil_multi_monitor_add(get_handle<sentil_multi_monitor_t>(prhs[1]), id.c_str(),
                                       formula.c_str()));
    } else if (cmd == "multi_monitor_add_formula") {
        need(nrhs, 4);
        std::string id = get_string(prhs[2]);
        check(sentil_multi_monitor_add_formula(get_handle<sentil_multi_monitor_t>(prhs[1]),
                                               id.c_str(), get_handle<sentil_formula_t>(prhs[3])));
    } else if (cmd == "multi_monitor_remove") {
        need(nrhs, 3);
        plhs[0] = mxCreateLogicalScalar(sentil_multi_monitor_remove(
            get_handle<sentil_multi_monitor_t>(prhs[1]), get_string(prhs[2]).c_str()));
    } else if (cmd == "multi_monitor_reset") {
        need(nrhs, 2);
        sentil_multi_monitor_reset(get_handle<sentil_multi_monitor_t>(prhs[1]));
    } else if (cmd == "multi_monitor_len") {
        need(nrhs, 2);
        plhs[0] = mxCreateDoubleScalar(static_cast<double>(
            sentil_multi_monitor_len(get_handle<sentil_multi_monitor_t>(prhs[1]))));
    } else if (cmd == "multi_monitor_is_empty") {
        need(nrhs, 2);
        plhs[0] = mxCreateLogicalScalar(
            sentil_multi_monitor_is_empty(get_handle<sentil_multi_monitor_t>(prhs[1])));
    } else if (cmd == "multi_monitor_ids") {
        need(nrhs, 2);
        size_t count = 0;
        char** ids = sentil_multi_monitor_ids(get_handle<sentil_multi_monitor_t>(prhs[1]), &count);
        plhs[0] = make_string_array(ids, count);
        if (ids != nullptr) {
            sentil_free_string_array(ids, count);
        }
    } else if (cmd == "multi_monitor_update") {
        need(nrhs, 5);
        sentil_multi_monitor_t* monitor = get_handle<sentil_multi_monitor_t>(prhs[1]);
        double time = get_scalar(prhs[2]);
        std::vector<std::string> storage;
        std::vector<const char*> names;
        get_string_cell(prhs[3], storage, names);
        size_t m = 0;
        const double* values = get_doubles(prhs[4], &m);
        if (m != names.size()) {
            fail("the names and values must have the same length");
        }
        size_t count = 0;
        sentil_named_robustness_t* result =
            sentil_multi_monitor_update(monitor, time, names.data(), values, m, &count);
        if (result == nullptr) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_named_robustness(result, count);
        sentil_free_named_robustness(result, count);
    } else if (cmd == "multi_monitor_destroy") {
        need(nrhs, 2);
        sentil_multi_monitor_destroy(get_handle<sentil_multi_monitor_t>(prhs[1]));
    } else if (cmd == "formula_bank_create") {
        plhs[0] = make_handle(checked(sentil_formula_bank_create()));
    } else if (cmd == "formula_bank_add") {
        need(nrhs, 4);
        std::string id = get_string(prhs[2]);
        std::string formula = get_string(prhs[3]);
        check(sentil_formula_bank_add(get_handle<sentil_formula_bank_t>(prhs[1]), id.c_str(),
                                      formula.c_str()));
    } else if (cmd == "formula_bank_add_formula") {
        need(nrhs, 4);
        std::string id = get_string(prhs[2]);
        check(sentil_formula_bank_add_formula(get_handle<sentil_formula_bank_t>(prhs[1]), id.c_str(),
                                              get_handle<sentil_formula_t>(prhs[3])));
    } else if (cmd == "formula_bank_ids") {
        need(nrhs, 2);
        size_t count = 0;
        char** ids = sentil_formula_bank_ids(get_handle<sentil_formula_bank_t>(prhs[1]), &count);
        plhs[0] = make_string_array(ids, count);
        if (ids != nullptr) {
            sentil_free_string_array(ids, count);
        }
    } else if (cmd == "formula_bank_len") {
        need(nrhs, 2);
        plhs[0] = mxCreateDoubleScalar(static_cast<double>(
            sentil_formula_bank_len(get_handle<sentil_formula_bank_t>(prhs[1]))));
    } else if (cmd == "formula_bank_is_empty") {
        need(nrhs, 2);
        plhs[0] = mxCreateLogicalScalar(
            sentil_formula_bank_is_empty(get_handle<sentil_formula_bank_t>(prhs[1])));
    } else if (cmd == "formula_bank_robustness") {
        need(nrhs, 3);
        size_t count = 0;
        sentil_bank_result_t* results = sentil_formula_bank_robustness(
            get_handle<sentil_formula_bank_t>(prhs[1]), get_handle<sentil_trace_t>(prhs[2]), &count);
        if (results == nullptr) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_bank(results, count);
    } else if (cmd == "formula_bank_robustness_dense") {
        need(nrhs, 3);
        size_t count = 0;
        sentil_bank_result_t* results = sentil_formula_bank_robustness_dense(
            get_handle<sentil_formula_bank_t>(prhs[1]), get_handle<sentil_trace_t>(prhs[2]), &count);
        if (results == nullptr) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_bank(results, count);
    } else if (cmd == "formula_bank_destroy") {
        need(nrhs, 2);
        sentil_formula_bank_destroy(get_handle<sentil_formula_bank_t>(prhs[1]));
    } else if (cmd == "stats_wilson") {
        need(nrhs, 4);
        plhs[0] = make_confidence(sentil_wilson_interval(static_cast<uint64_t>(get_scalar(prhs[1])),
                                                         static_cast<uint64_t>(get_scalar(prhs[2])),
                                                         get_scalar(prhs[3])));
    } else if (cmd == "stats_clopper_pearson") {
        need(nrhs, 4);
        plhs[0] = make_confidence(sentil_clopper_pearson(static_cast<uint64_t>(get_scalar(prhs[1])),
                                                         static_cast<uint64_t>(get_scalar(prhs[2])),
                                                         get_scalar(prhs[3])));
    } else if (cmd == "stats_jeffreys") {
        need(nrhs, 4);
        plhs[0] = make_confidence(sentil_jeffreys_interval(
            static_cast<uint64_t>(get_scalar(prhs[1])), static_cast<uint64_t>(get_scalar(prhs[2])),
            get_scalar(prhs[3])));
    } else if (cmd == "stats_agresti_coull") {
        need(nrhs, 4);
        plhs[0] = make_confidence(sentil_agresti_coull(static_cast<uint64_t>(get_scalar(prhs[1])),
                                                       static_cast<uint64_t>(get_scalar(prhs[2])),
                                                       get_scalar(prhs[3])));
    } else if (cmd == "stats_interval") {
        need(nrhs, 5);
        plhs[0] = make_confidence(sentil_interval(
            static_cast<sentil_interval_method_t>(static_cast<int>(get_scalar(prhs[1]))),
            static_cast<uint64_t>(get_scalar(prhs[2])), static_cast<uint64_t>(get_scalar(prhs[3])),
            get_scalar(prhs[4])));
    } else if (cmd == "stats_z_score") {
        need(nrhs, 2);
        plhs[0] = mxCreateDoubleScalar(sentil_z_score(get_scalar(prhs[1])));
    } else if (cmd == "stats_chernoff_hoeffding") {
        need(nrhs, 3);
        uint64_t out = 0;
        check(sentil_chernoff_hoeffding_samples(get_scalar(prhs[1]), get_scalar(prhs[2]), &out));
        plhs[0] = mxCreateDoubleScalar(static_cast<double>(out));
    } else if (cmd == "stats_wilson_samples") {
        need(nrhs, 3);
        uint64_t out = 0;
        check(sentil_wilson_samples(get_scalar(prhs[1]), get_scalar(prhs[2]), &out));
        plhs[0] = mxCreateDoubleScalar(static_cast<double>(out));
    } else if (cmd == "noise_dirac") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_noise_dirac(get_scalar(prhs[1]))));
    } else if (cmd == "noise_gaussian") {
        need(nrhs, 3);
        plhs[0] = make_handle(checked(sentil_noise_gaussian(get_scalar(prhs[1]), get_scalar(prhs[2]))));
    } else if (cmd == "noise_uniform") {
        need(nrhs, 3);
        plhs[0] = make_handle(checked(sentil_noise_uniform(get_scalar(prhs[1]), get_scalar(prhs[2]))));
    } else if (cmd == "noise_log_normal") {
        need(nrhs, 3);
        plhs[0] =
            make_handle(checked(sentil_noise_log_normal(get_scalar(prhs[1]), get_scalar(prhs[2]))));
    } else if (cmd == "noise_exponential") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_noise_exponential(get_scalar(prhs[1]))));
    } else if (cmd == "noise_gamma") {
        need(nrhs, 3);
        plhs[0] = make_handle(checked(sentil_noise_gamma(get_scalar(prhs[1]), get_scalar(prhs[2]))));
    } else if (cmd == "noise_beta") {
        need(nrhs, 3);
        plhs[0] = make_handle(checked(sentil_noise_beta(get_scalar(prhs[1]), get_scalar(prhs[2]))));
    } else if (cmd == "noise_weibull") {
        need(nrhs, 3);
        plhs[0] = make_handle(checked(sentil_noise_weibull(get_scalar(prhs[1]), get_scalar(prhs[2]))));
    } else if (cmd == "noise_rayleigh") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_noise_rayleigh(get_scalar(prhs[1]))));
    } else if (cmd == "noise_gumbel") {
        need(nrhs, 3);
        plhs[0] = make_handle(checked(sentil_noise_gumbel(get_scalar(prhs[1]), get_scalar(prhs[2]))));
    } else if (cmd == "noise_cauchy") {
        need(nrhs, 3);
        plhs[0] = make_handle(checked(sentil_noise_cauchy(get_scalar(prhs[1]), get_scalar(prhs[2]))));
    } else if (cmd == "noise_student_t") {
        need(nrhs, 4);
        plhs[0] = make_handle(checked(
            sentil_noise_student_t(get_scalar(prhs[1]), get_scalar(prhs[2]), get_scalar(prhs[3]))));
    } else if (cmd == "noise_truncated_normal") {
        need(nrhs, 5);
        plhs[0] = make_handle(checked(sentil_noise_truncated_normal(
            get_scalar(prhs[1]), get_scalar(prhs[2]), get_scalar(prhs[3]), get_scalar(prhs[4]))));
    } else if (cmd == "noise_poisson") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_noise_poisson(get_scalar(prhs[1]))));
    } else if (cmd == "noise_binomial") {
        need(nrhs, 3);
        plhs[0] = make_handle(checked(
            sentil_noise_binomial(static_cast<uint64_t>(get_scalar(prhs[1])), get_scalar(prhs[2]))));
    } else if (cmd == "noise_bootstrap") {
        need(nrhs, 2);
        size_t n = 0;
        const double* residuals = get_doubles(prhs[1], &n);
        plhs[0] = make_handle(checked(sentil_noise_bootstrap(residuals, n)));
    } else if (cmd == "noise_mixture") {
        need(nrhs, 3);
        size_t wn = 0;
        const double* weights = get_doubles(prhs[1], &wn);
        if (!mxIsUint64(prhs[2])) {
            fail("expected an array of model handles");
        }
        size_t mn = mxGetNumberOfElements(prhs[2]);
        if (wn != mn) {
            fail("the weights and models must have the same length");
        }
        uint64_t* raw = static_cast<uint64_t*>(mxGetData(prhs[2]));
        std::vector<sentil_noise_model_t*> models(mn);
        for (size_t i = 0; i < mn; ++i) {
            models[i] = reinterpret_cast<sentil_noise_model_t*>(raw[i]);
        }
        plhs[0] = make_handle(checked(sentil_noise_mixture(weights, models.data(), mn)));
    } else if (cmd == "noise_mean") {
        need(nrhs, 2);
        double out = 0.0;
        bool defined = sentil_noise_mean(get_handle<sentil_noise_model_t>(prhs[1]), &out);
        plhs[0] = defined ? mxCreateDoubleScalar(out) : mxCreateDoubleMatrix(1, 0, mxREAL);
    } else if (cmd == "noise_variance") {
        need(nrhs, 2);
        double out = 0.0;
        bool defined = sentil_noise_variance(get_handle<sentil_noise_model_t>(prhs[1]), &out);
        plhs[0] = defined ? mxCreateDoubleScalar(out) : mxCreateDoubleMatrix(1, 0, mxREAL);
    } else if (cmd == "noise_residuals") {
        need(nrhs, 4);
        size_t n = 0, m = 0;
        const double* truth = get_doubles(prhs[1], &n);
        const double* sensor = get_doubles(prhs[2], &m);
        size_t len = 0;
        double* residuals = sentil_noise_residuals(
            truth, n, sensor, m, static_cast<sentil_noise_interaction_t>(static_cast<int>(get_scalar(prhs[3]))),
            &len);
        if (residuals == nullptr) {
            raise_last(sentil_get_last_error_code());
        }
        plhs[0] = make_doubles(residuals, len);
        sentil_free_doubles(residuals, len);
    } else if (cmd == "noise_fit_gaussian") {
        need(nrhs, 2);
        size_t n = 0;
        const double* samples = get_doubles(prhs[1], &n);
        plhs[0] = make_handle(checked(sentil_noise_fit_gaussian(samples, n)));
    } else if (cmd == "noise_fit_bootstrap") {
        need(nrhs, 2);
        size_t n = 0;
        const double* samples = get_doubles(prhs[1], &n);
        plhs[0] = make_handle(checked(sentil_noise_fit_bootstrap(samples, n)));
    } else if (cmd == "noise_fit_bootstrap_reservoir") {
        need(nrhs, 3);
        size_t n = 0;
        const double* samples = get_doubles(prhs[1], &n);
        plhs[0] = make_handle(checked(sentil_noise_fit_bootstrap_reservoir(
            samples, n, static_cast<size_t>(get_scalar(prhs[2])))));
    } else if (cmd == "noise_fit_gaussian_mixture") {
        need(nrhs, 4);
        size_t n = 0;
        const double* samples = get_doubles(prhs[1], &n);
        plhs[0] = make_handle(checked(sentil_noise_fit_gaussian_mixture(
            samples, n, static_cast<size_t>(get_scalar(prhs[2])),
            static_cast<size_t>(get_scalar(prhs[3])))));
    } else if (cmd == "noise_to_json") {
        need(nrhs, 2);
        char* json = sentil_noise_to_json(get_handle<sentil_noise_model_t>(prhs[1]));
        plhs[0] = make_string(json);
        sentil_free_string(json);
    } else if (cmd == "noise_from_json") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_noise_from_json(get_string(prhs[1]).c_str())));
    } else if (cmd == "noise_from_file") {
        need(nrhs, 2);
        plhs[0] = make_handle(checked(sentil_noise_from_file(get_string(prhs[1]).c_str())));
    } else if (cmd == "noise_destroy") {
        need(nrhs, 2);
        sentil_noise_destroy(get_handle<sentil_noise_model_t>(prhs[1]));
    } else if (cmd == "lifting_create") {
        plhs[0] = make_handle(checked(sentil_lifting_registry_create()));
    } else if (cmd == "lifting_register") {
        need(nrhs, 5);
        std::string variable = get_string(prhs[2]);
        check(sentil_lifting_registry_register(
            get_handle<sentil_lifting_registry_t>(prhs[1]), variable.c_str(),
            get_handle<sentil_noise_model_t>(prhs[3]),
            static_cast<sentil_noise_interaction_t>(static_cast<int>(get_scalar(prhs[4])))));
    } else if (cmd == "lifting_variables") {
        need(nrhs, 2);
        size_t count = 0;
        char** names =
            sentil_lifting_registry_variables(get_handle<sentil_lifting_registry_t>(prhs[1]), &count);
        plhs[0] = make_string_array(names, count);
        if (names != nullptr) {
            sentil_free_string_array(names, count);
        }
    } else if (cmd == "lifting_is_empty") {
        need(nrhs, 2);
        plhs[0] = mxCreateLogicalScalar(
            sentil_lifting_registry_is_empty(get_handle<sentil_lifting_registry_t>(prhs[1])));
    } else if (cmd == "lifting_lift") {
        need(nrhs, 4);
        plhs[0] = make_handle(checked(sentil_lifting_registry_lift(
            get_handle<sentil_lifting_registry_t>(prhs[1]), get_handle<sentil_trace_t>(prhs[2]),
            static_cast<uint64_t>(get_scalar(prhs[3])))));
    } else if (cmd == "lifting_destroy") {
        need(nrhs, 2);
        sentil_lifting_registry_destroy(get_handle<sentil_lifting_registry_t>(prhs[1]));
    } else {
        fail("unknown command: " + cmd);
    }
}