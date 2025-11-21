/* Command-dispatch gateway for the +sentil package. */

#include "mex.h"
#include "sentil.h"

#include <cstdint>
#include <string>
#include <vector>

namespace {

/* Raise the engine's last error as a typed MATLAB error so a caller can branch on the
 * identifier, mirroring the Python SentilError subclasses and the Java exceptions. */
void raise_last(sentil_error_t code) {
    size_t needed = sentil_get_last_error_message(nullptr, 0);
    std::string message;
    if (needed > 0) {
        message.resize(needed);
        sentil_get_last_error_message(&message[0], needed);
        message.resize(needed - 1);
    }
    const char* id = "sentil:evaluation";
    switch (code) {
        case SENTIL_ERR_PARSE:
            id = "sentil:parse";
            break;
        case SENTIL_ERR_UNKNOWN_VARIABLE:
        case SENTIL_ERR_NOT_PROBABILISTIC:
        case SENTIL_ERR_UNSUPPORTED:
            id = "sentil:semantic";
            break;
        default:
            break;
    }
    mexErrMsgIdAndTxt(id, "%s", message.empty() ? "sentil error" : message.c_str());
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
    } else {
        fail("unknown command: " + cmd);
    }
}