#include "io_github_sedislab_sentil_NativeLib.h"

#include <sentil.h>

#include <cstdint>
#include <string>
#include <vector>

namespace {

jclass cls_base = nullptr;
jclass cls_parse = nullptr;
jclass cls_semantic = nullptr;
jclass cls_evaluation = nullptr;
jmethodID ctor_base = nullptr;
jmethodID ctor_parse = nullptr;
jmethodID ctor_semantic = nullptr;
jmethodID ctor_evaluation = nullptr;
jclass cls_string = nullptr;

jclass cls_sample = nullptr;
jmethodID ctor_sample = nullptr;
jclass cls_robustness = nullptr;
jmethodID ctor_robustness = nullptr;
jclass cls_interval = nullptr;
jmethodID ctor_interval = nullptr;

// sentil_get_last_error_message sizes on a null first call and counts the NUL.
std::string last_error_message() {
    size_t needed = sentil_get_last_error_message(nullptr, 0);
    if (needed <= 1) {
        return std::string();
    }
    std::string buffer(needed, '\0');
    sentil_get_last_error_message(&buffer[0], needed);
    buffer.resize(needed - 1);
    return buffer;
}

void throw_sentil(JNIEnv* env, sentil_error_t code, const std::string& message) {
    jclass cls = cls_evaluation;
    jmethodID ctor = ctor_evaluation;
    switch (code) {
        case SENTIL_ERR_PARSE:
            cls = cls_parse;
            ctor = ctor_parse;
            break;
        case SENTIL_ERR_UNKNOWN_VARIABLE:
        case SENTIL_ERR_NOT_PROBABILISTIC:
        case SENTIL_ERR_UNSUPPORTED:
            cls = cls_semantic;
            ctor = ctor_semantic;
            break;
        default:
            break;
    }
    jstring jmsg = env->NewStringUTF(message.c_str());
    jobject ex = env->NewObject(cls, ctor, jmsg, static_cast<jint>(code));
    if (ex != nullptr) {
        env->Throw(static_cast<jthrowable>(ex));
    }
    if (jmsg != nullptr) {
        env->DeleteLocalRef(jmsg);
    }
}

void raise_last(JNIEnv* env) {
    sentil_error_t code = sentil_get_last_error_code();
    if (code == SENTIL_OK) {
        code = SENTIL_ERR_EVALUATION;
    }
    std::string message = last_error_message();
    if (message.empty()) {
        message = "SENTIL error";
    }
    throw_sentil(env, code, message);
}

struct Utf8 {
    JNIEnv* env;
    jstring str;
    const char* chars;

    Utf8(JNIEnv* env, jstring str) : env(env), str(str) {
        chars = str != nullptr ? env->GetStringUTFChars(str, nullptr) : nullptr;
    }
    ~Utf8() {
        if (str != nullptr && chars != nullptr) {
            env->ReleaseStringUTFChars(str, chars);
        }
    }
    const char* c() const { return chars; }
};

template <typename T>
T* as_ptr(jlong handle) {
    return reinterpret_cast<T*>(handle);
}

jstring owned_string(JNIEnv* env, char* owned) {
    jstring result = env->NewStringUTF(owned);
    sentil_free_string(owned);
    return result;
}

jobjectArray owned_string_array(JNIEnv* env, char** owned, size_t count) {
    jobjectArray result = env->NewObjectArray(static_cast<jsize>(count), cls_string, nullptr);
    if (result != nullptr) {
        for (size_t i = 0; i < count; ++i) {
            jstring element = env->NewStringUTF(owned[i]);
            env->SetObjectArrayElement(result, static_cast<jsize>(i), element);
            env->DeleteLocalRef(element);
        }
    }
    sentil_free_string_array(owned, count);
    return result;
}

bool failed(JNIEnv* env, sentil_error_t code) {
    if (code == SENTIL_OK) {
        return false;
    }
    std::string message = last_error_message();
    if (message.empty()) {
        message = "SENTIL error";
    }
    throw_sentil(env, code, message);
    return true;
}

struct DoubleArray {
    JNIEnv* env;
    jdoubleArray array;
    jdouble* elements;
    jsize length;

    DoubleArray(JNIEnv* env, jdoubleArray array) : env(env), array(array) {
        if (array != nullptr) {
            length = env->GetArrayLength(array);
            elements = env->GetDoubleArrayElements(array, nullptr);
        } else {
            length = 0;
            elements = nullptr;
        }
    }
    ~DoubleArray() {
        if (array != nullptr && elements != nullptr) {
            env->ReleaseDoubleArrayElements(array, elements, JNI_ABORT);
        }
    }
    const double* data() const { return elements; }
    size_t size() const { return static_cast<size_t>(length); }
};

jdoubleArray copy_doubles(JNIEnv* env, const double* data, size_t count) {
    jdoubleArray result = env->NewDoubleArray(static_cast<jsize>(count));
    if (result != nullptr && count > 0) {
        env->SetDoubleArrayRegion(result, 0, static_cast<jsize>(count), data);
    }
    return result;
}

jdoubleArray owned_doubles(JNIEnv* env, double* data, size_t count) {
    jdoubleArray result = copy_doubles(env, data, count);
    sentil_free_doubles(data, count);
    return result;
}

bool cache_class_ctor(JNIEnv* env, const char* name, const char* sig, jclass* cls,
                      jmethodID* ctor) {
    jclass local = env->FindClass(name);
    if (local == nullptr) {
        return false;
    }
    *cls = static_cast<jclass>(env->NewGlobalRef(local));
    env->DeleteLocalRef(local);
    *ctor = env->GetMethodID(*cls, "<init>", sig);
    return *ctor != nullptr;
}

jobject make_sample(JNIEnv* env, const sentil_sample_t& sample) {
    return env->NewObject(cls_sample, ctor_sample, sample.found ? JNI_TRUE : JNI_FALSE, sample.time,
                          sample.value);
}

jdoubleArray optional_double(JNIEnv* env, bool present, double value) {
    jdoubleArray result = env->NewDoubleArray(present ? 1 : 0);
    if (present && result != nullptr) {
        env->SetDoubleArrayRegion(result, 0, 1, &value);
    }
    return result;
}

jobjectArray owned_sample_array(JNIEnv* env, sentil_sample_t* owned, size_t count) {
    jobjectArray result = env->NewObjectArray(static_cast<jsize>(count), cls_sample, nullptr);
    if (result != nullptr) {
        for (size_t i = 0; i < count; ++i) {
            jobject element = make_sample(env, owned[i]);
            env->SetObjectArrayElement(result, static_cast<jsize>(i), element);
            env->DeleteLocalRef(element);
        }
    }
    sentil_free_samples(owned, count);
    return result;
}

jobject make_robustness(JNIEnv* env, const sentil_robustness_t& r) {
    return env->NewObject(cls_robustness, ctor_robustness, r.resolved ? JNI_TRUE : JNI_FALSE,
                          r.satisfied ? JNI_TRUE : JNI_FALSE, r.value, r.lower, r.upper);
}

jobjectArray owned_robustness_array(JNIEnv* env, sentil_robustness_t* owned, size_t count) {
    jobjectArray result = env->NewObjectArray(static_cast<jsize>(count), cls_robustness, nullptr);
    if (result != nullptr) {
        for (size_t i = 0; i < count; ++i) {
            jobject element = make_robustness(env, owned[i]);
            env->SetObjectArrayElement(result, static_cast<jsize>(i), element);
            env->DeleteLocalRef(element);
        }
    }
    sentil_free_robustness(owned, count);
    return result;
}

jobjectArray owned_interval_array(JNIEnv* env, sentil_interval_t* owned, size_t count) {
    jobjectArray result = env->NewObjectArray(static_cast<jsize>(count), cls_interval, nullptr);
    if (result != nullptr) {
        for (size_t i = 0; i < count; ++i) {
            jobject element = env->NewObject(cls_interval, ctor_interval, owned[i].start,
                                             owned[i].end);
            env->SetObjectArrayElement(result, static_cast<jsize>(i), element);
            env->DeleteLocalRef(element);
        }
    }
    sentil_free_intervals(owned, count);
    return result;
}

struct StringArray {
    JNIEnv* env;
    jobjectArray array;
    std::vector<jstring> strings;
    std::vector<const char*> chars;

    StringArray(JNIEnv* env, jobjectArray array) : env(env), array(array) {
        if (array != nullptr) {
            jsize count = env->GetArrayLength(array);
            // Every element stays a live local ref until the destructor, and the JNI
            // spec only guarantees room for 16, so ask for the whole run up front.
            env->EnsureLocalCapacity(count);
            strings.reserve(static_cast<size_t>(count));
            chars.reserve(static_cast<size_t>(count));
            for (jsize i = 0; i < count; ++i) {
                jstring element = static_cast<jstring>(env->GetObjectArrayElement(array, i));
                strings.push_back(element);
                chars.push_back(element != nullptr ? env->GetStringUTFChars(element, nullptr)
                                                   : nullptr);
            }
        }
    }
    ~StringArray() {
        for (size_t i = 0; i < strings.size(); ++i) {
            if (strings[i] != nullptr && chars[i] != nullptr) {
                env->ReleaseStringUTFChars(strings[i], chars[i]);
            }
            if (strings[i] != nullptr) {
                env->DeleteLocalRef(strings[i]);
            }
        }
    }
    const char* const* data() const { return chars.data(); }
    size_t size() const { return chars.size(); }
};

}  // namespace

JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM* vm, void*) {
    JNIEnv* env = nullptr;
    if (vm->GetEnv(reinterpret_cast<void**>(&env), JNI_VERSION_1_8) != JNI_OK) {
        return JNI_ERR;
    }
    const char* names[] = {
        "io/github/sedislab/sentil/SentilException",
        "io/github/sedislab/sentil/ParseException",
        "io/github/sedislab/sentil/SemanticException",
        "io/github/sedislab/sentil/EvaluationException",
    };
    jclass* slots[] = {&cls_base, &cls_parse, &cls_semantic, &cls_evaluation};
    jmethodID* ctors[] = {&ctor_base, &ctor_parse, &ctor_semantic, &ctor_evaluation};
    for (int i = 0; i < 4; ++i) {
        jclass local = env->FindClass(names[i]);
        if (local == nullptr) {
            return JNI_ERR;
        }
        *slots[i] = static_cast<jclass>(env->NewGlobalRef(local));
        env->DeleteLocalRef(local);
        *ctors[i] = env->GetMethodID(*slots[i], "<init>", "(Ljava/lang/String;I)V");
        if (*ctors[i] == nullptr) {
            return JNI_ERR;
        }
    }
    jclass string_local = env->FindClass("java/lang/String");
    if (string_local == nullptr) {
        return JNI_ERR;
    }
    cls_string = static_cast<jclass>(env->NewGlobalRef(string_local));
    env->DeleteLocalRef(string_local);
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/Sample", "(ZDD)V", &cls_sample,
                          &ctor_sample)) {
        return JNI_ERR;
    }
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/Robustness", "(ZZDDD)V", &cls_robustness,
                          &ctor_robustness)) {
        return JNI_ERR;
    }
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/Interval", "(DD)V", &cls_interval,
                          &ctor_interval)) {
        return JNI_ERR;
    }
    return JNI_VERSION_1_8;
}

JNIEXPORT void JNICALL JNI_OnUnload(JavaVM* vm, void*) {
    JNIEnv* env = nullptr;
    if (vm->GetEnv(reinterpret_cast<void**>(&env), JNI_VERSION_1_8) != JNI_OK) {
        return;
    }
    for (jclass* slot : {&cls_base, &cls_parse, &cls_semantic, &cls_evaluation, &cls_string,
                         &cls_sample, &cls_robustness, &cls_interval}) {
        if (*slot != nullptr) {
            env->DeleteGlobalRef(*slot);
            *slot = nullptr;
        }
    }
}

JNIEXPORT jintArray JNICALL Java_io_github_sedislab_sentil_NativeLib_version(JNIEnv* env, jclass) {
    uint32_t major = 0;
    uint32_t minor = 0;
    uint32_t patch = 0;
    sentil_version(&major, &minor, &patch);
    jint parts[3] = {static_cast<jint>(major), static_cast<jint>(minor), static_cast<jint>(patch)};
    jintArray out = env->NewIntArray(3);
    if (out != nullptr) {
        env->SetIntArrayRegion(out, 0, 3, parts);
    }
    return out;
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaParse(JNIEnv* env, jclass,
                                                                             jstring formula) {
    Utf8 text(env, formula);
    sentil_formula_t* parsed = sentil_formula_parse(text.c());
    if (parsed == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(parsed);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaDestroy(JNIEnv*, jclass,
                                                                              jlong handle) {
    sentil_formula_destroy(as_ptr<sentil_formula_t>(handle));
}

JNIEXPORT jstring JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaToJson(JNIEnv* env, jclass,
                                                                                jlong handle) {
    char* json = sentil_formula_to_json(as_ptr<const sentil_formula_t>(handle));
    if (json == nullptr) {
        raise_last(env);
        return nullptr;
    }
    return owned_string(env, json);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaFromJson(JNIEnv* env, jclass,
                                                                                jstring json) {
    Utf8 text(env, json);
    sentil_formula_t* parsed = sentil_formula_from_json(text.c());
    if (parsed == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(parsed);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaDepth(JNIEnv*, jclass,
                                                                             jlong handle) {
    return static_cast<jlong>(sentil_formula_depth(as_ptr<const sentil_formula_t>(handle)));
}

JNIEXPORT jboolean JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaHasTemporal(JNIEnv*,
                                                                                      jclass,
                                                                                      jlong handle) {
    return sentil_formula_has_temporal(as_ptr<const sentil_formula_t>(handle)) ? JNI_TRUE
                                                                               : JNI_FALSE;
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaVariables(
    JNIEnv* env, jclass, jlong handle) {
    size_t count = 0;
    char** vars = sentil_formula_variables(as_ptr<const sentil_formula_t>(handle), &count);
    if (vars == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObjectArray(0, cls_string, nullptr);
    }
    return owned_string_array(env, vars, count);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_traceCreate(JNIEnv* env, jclass,
                                                                            jdoubleArray times) {
    DoubleArray t(env, times);
    sentil_trace_t* trace = sentil_trace_create(t.data(), t.size());
    if (trace == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(trace);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_traceFromSignal(
    JNIEnv* env, jclass, jdoubleArray times, jstring name, jdoubleArray values) {
    DoubleArray t(env, times);
    Utf8 signal_name(env, name);
    DoubleArray v(env, values);
    sentil_trace_t* trace =
        sentil_trace_from_signal(t.data(), t.size(), signal_name.c(), v.data(), v.size());
    if (trace == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(trace);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_traceIndexed(JNIEnv* env, jclass,
                                                                             jlong length) {
    sentil_trace_t* trace = sentil_trace_indexed(static_cast<size_t>(length));
    if (trace == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(trace);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_traceAddSignal(
    JNIEnv* env, jclass, jlong handle, jstring name, jdoubleArray values) {
    Utf8 signal_name(env, name);
    DoubleArray v(env, values);
    sentil_error_t code = sentil_trace_add_signal(as_ptr<sentil_trace_t>(handle), signal_name.c(),
                                                  v.data(), v.size());
    failed(env, code);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_traceLen(JNIEnv*, jclass,
                                                                         jlong handle) {
    return static_cast<jlong>(sentil_trace_len(as_ptr<const sentil_trace_t>(handle)));
}

JNIEXPORT jboolean JNICALL Java_io_github_sedislab_sentil_NativeLib_traceIsEmpty(JNIEnv*, jclass,
                                                                                jlong handle) {
    return sentil_trace_is_empty(as_ptr<const sentil_trace_t>(handle)) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_traceTimes(JNIEnv* env,
                                                                                  jclass,
                                                                                  jlong handle) {
    size_t length = 0;
    const double* times = sentil_trace_times(as_ptr<const sentil_trace_t>(handle), &length);
    if (times == nullptr) {
        return copy_doubles(env, nullptr, 0);
    }
    return copy_doubles(env, times, length);
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_traceVariables(
    JNIEnv* env, jclass, jlong handle) {
    size_t count = 0;
    char** vars = sentil_trace_variables(as_ptr<const sentil_trace_t>(handle), &count);
    if (vars == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObjectArray(0, cls_string, nullptr);
    }
    return owned_string_array(env, vars, count);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_traceSignal(JNIEnv* env,
                                                                                   jclass,
                                                                                   jlong handle,
                                                                                   jstring name) {
    Utf8 signal_name(env, name);
    size_t length = 0;
    const double* signal =
        sentil_trace_signal(as_ptr<const sentil_trace_t>(handle), signal_name.c(), &length);
    if (signal == nullptr) {
        return nullptr;
    }
    return copy_doubles(env, signal, length);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_traceDestroy(JNIEnv*, jclass,
                                                                            jlong handle) {
    sentil_trace_destroy(as_ptr<sentil_trace_t>(handle));
}

JNIEXPORT jdouble JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaRobustness(
    JNIEnv* env, jclass, jlong formula, jlong trace) {
    double out = 0.0;
    sentil_error_t code = sentil_formula_robustness(as_ptr<const sentil_formula_t>(formula),
                                                    as_ptr<const sentil_trace_t>(trace), &out);
    if (failed(env, code)) {
        return 0.0;
    }
    return out;
}

JNIEXPORT jdouble JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaRobustnessDense(
    JNIEnv* env, jclass, jlong formula, jlong trace) {
    double out = 0.0;
    sentil_error_t code = sentil_formula_robustness_dense(as_ptr<const sentil_formula_t>(formula),
                                                          as_ptr<const sentil_trace_t>(trace), &out);
    if (failed(env, code)) {
        return 0.0;
    }
    return out;
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaRobustnessSignal(
    JNIEnv* env, jclass, jlong formula, jlong trace) {
    size_t length = 0;
    double* signal = sentil_formula_robustness_signal(as_ptr<const sentil_formula_t>(formula),
                                                      as_ptr<const sentil_trace_t>(trace), &length);
    if (signal == nullptr) {
        raise_last(env);
        return nullptr;
    }
    return owned_doubles(env, signal, length);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaRobustnessDenseSignal(
    JNIEnv* env, jclass, jlong formula, jlong trace) {
    size_t length = 0;
    double* signal = sentil_formula_robustness_dense_signal(
        as_ptr<const sentil_formula_t>(formula), as_ptr<const sentil_trace_t>(trace), &length);
    if (signal == nullptr) {
        raise_last(env);
        return nullptr;
    }
    return owned_doubles(env, signal, length);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_exprVariable(JNIEnv* env, jclass,
                                                                             jstring name) {
    Utf8 variable(env, name);
    sentil_expr_t* expr = sentil_expr_variable(variable.c());
    if (expr == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(expr);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_exprLiteral(JNIEnv* env, jclass,
                                                                            jdouble value) {
    sentil_expr_t* expr = sentil_expr_literal(value);
    if (expr == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(expr);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_exprBinary(JNIEnv* env, jclass,
                                                                           jint op, jlong left,
                                                                           jlong right) {
    sentil_expr_t* expr = sentil_expr_binary(static_cast<sentil_binary_op_t>(op),
                                             as_ptr<sentil_expr_t>(left),
                                             as_ptr<sentil_expr_t>(right));
    if (expr == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(expr);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_exprCall(JNIEnv* env, jclass,
                                                                         jstring name,
                                                                         jlongArray args) {
    Utf8 function(env, name);
    jsize count = env->GetArrayLength(args);
    jlong* handles = env->GetLongArrayElements(args, nullptr);
    std::vector<sentil_expr_t*> operands(static_cast<size_t>(count));
    for (jsize i = 0; i < count; ++i) {
        operands[static_cast<size_t>(i)] = as_ptr<sentil_expr_t>(handles[i]);
    }
    env->ReleaseLongArrayElements(args, handles, JNI_ABORT);
    sentil_expr_t* expr =
        sentil_expr_call(function.c(), operands.data(), static_cast<size_t>(count));
    if (expr == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(expr);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_exprDestroy(JNIEnv*, jclass,
                                                                           jlong handle) {
    sentil_expr_destroy(as_ptr<sentil_expr_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaPredicate(JNIEnv* env, jclass,
                                                                                 jlong left, jint op,
                                                                                 jlong right) {
    sentil_formula_t* formula =
        sentil_formula_predicate(as_ptr<sentil_expr_t>(left),
                                 static_cast<sentil_comparison_op_t>(op),
                                 as_ptr<sentil_expr_t>(right));
    if (formula == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(formula);
}

static jlong return_formula(JNIEnv* env, sentil_formula_t* formula) {
    if (formula == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(formula);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaNot(JNIEnv* env, jclass,
                                                                           jlong child) {
    return return_formula(env, sentil_formula_not(as_ptr<sentil_formula_t>(child)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaAnd(JNIEnv* env, jclass,
                                                                           jlong left, jlong right) {
    return return_formula(env, sentil_formula_and(as_ptr<sentil_formula_t>(left),
                                                  as_ptr<sentil_formula_t>(right)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaOr(JNIEnv* env, jclass,
                                                                          jlong left, jlong right) {
    return return_formula(env, sentil_formula_or(as_ptr<sentil_formula_t>(left),
                                                 as_ptr<sentil_formula_t>(right)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaImplies(JNIEnv* env, jclass,
                                                                               jlong left,
                                                                               jlong right) {
    return return_formula(env, sentil_formula_implies(as_ptr<sentil_formula_t>(left),
                                                      as_ptr<sentil_formula_t>(right)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaNext(JNIEnv* env, jclass,
                                                                            jlong child) {
    return return_formula(env, sentil_formula_next(as_ptr<sentil_formula_t>(child)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaAlways(
    JNIEnv* env, jclass, jdouble lower, jdouble upper, jboolean has_upper, jlong child) {
    return return_formula(env, sentil_formula_always(lower, upper, has_upper == JNI_TRUE,
                                                     as_ptr<sentil_formula_t>(child)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaEventually(
    JNIEnv* env, jclass, jdouble lower, jdouble upper, jboolean has_upper, jlong child) {
    return return_formula(env, sentil_formula_eventually(lower, upper, has_upper == JNI_TRUE,
                                                         as_ptr<sentil_formula_t>(child)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaHistorically(
    JNIEnv* env, jclass, jdouble lower, jdouble upper, jboolean has_upper, jlong child) {
    return return_formula(env, sentil_formula_historically(lower, upper, has_upper == JNI_TRUE,
                                                           as_ptr<sentil_formula_t>(child)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaOnce(
    JNIEnv* env, jclass, jdouble lower, jdouble upper, jboolean has_upper, jlong child) {
    return return_formula(env, sentil_formula_once(lower, upper, has_upper == JNI_TRUE,
                                                   as_ptr<sentil_formula_t>(child)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaUntil(
    JNIEnv* env, jclass, jdouble lower, jdouble upper, jboolean has_upper, jlong left,
    jlong right) {
    return return_formula(env, sentil_formula_until(lower, upper, has_upper == JNI_TRUE,
                                                    as_ptr<sentil_formula_t>(left),
                                                    as_ptr<sentil_formula_t>(right)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaSince(
    JNIEnv* env, jclass, jdouble lower, jdouble upper, jboolean has_upper, jlong left,
    jlong right) {
    return return_formula(env, sentil_formula_since(lower, upper, has_upper == JNI_TRUE,
                                                    as_ptr<sentil_formula_t>(left),
                                                    as_ptr<sentil_formula_t>(right)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaProbabilistic(
    JNIEnv* env, jclass, jint op, jdouble threshold, jlong child) {
    return return_formula(env, sentil_formula_probabilistic(
                                   static_cast<sentil_probability_op_t>(op), threshold,
                                   as_ptr<sentil_formula_t>(child)));
}

static jlong return_trace(JNIEnv* env, sentil_trace_t* trace) {
    if (trace == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(trace);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_traceResample(
    JNIEnv* env, jclass, jlong handle, jdoubleArray times, jint interp) {
    DoubleArray grid(env, times);
    return return_trace(env, sentil_trace_resample(as_ptr<const sentil_trace_t>(handle),
                                                   grid.data(), grid.size(),
                                                   static_cast<sentil_interpolation_t>(interp)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_tracePrepare(JNIEnv* env, jclass,
                                                                             jlong handle,
                                                                             jint interp) {
    sentil_prepared_trace_t* prepared = sentil_trace_prepare(
        as_ptr<const sentil_trace_t>(handle), static_cast<sentil_interpolation_t>(interp));
    if (prepared == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(prepared);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_preparedTraceResample(
    JNIEnv* env, jclass, jlong prepared, jdoubleArray times) {
    DoubleArray grid(env, times);
    return return_trace(env, sentil_prepared_trace_resample(
                                 as_ptr<const sentil_prepared_trace_t>(prepared), grid.data(),
                                 grid.size()));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_preparedTraceDestroy(JNIEnv*, jclass,
                                                                                    jlong handle) {
    sentil_prepared_trace_destroy(as_ptr<sentil_prepared_trace_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_traceFromCsv(JNIEnv* env, jclass,
                                                                             jstring text) {
    Utf8 source(env, text);
    return return_trace(env, sentil_trace_from_csv(source.c()));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_traceFromTsv(JNIEnv* env, jclass,
                                                                             jstring text) {
    Utf8 source(env, text);
    return return_trace(env, sentil_trace_from_tsv(source.c()));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_traceFromPath(JNIEnv* env, jclass,
                                                                              jstring path) {
    Utf8 location(env, path);
    return return_trace(env, sentil_trace_from_path(location.c()));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferCreate(JNIEnv* env, jclass,
                                                                                 jlong capacity) {
    sentil_ring_buffer_t* buffer = sentil_ring_buffer_create(static_cast<size_t>(capacity));
    if (buffer == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(buffer);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferPush(
    JNIEnv* env, jclass, jlong handle, jdouble time, jdouble value) {
    sentil_sample_t evicted = {false, 0.0, 0.0};
    sentil_error_t code =
        sentil_ring_buffer_push(as_ptr<sentil_ring_buffer_t>(handle), time, value, &evicted);
    if (failed(env, code)) {
        return nullptr;
    }
    return make_sample(env, evicted);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferClear(JNIEnv*, jclass,
                                                                               jlong handle) {
    sentil_ring_buffer_clear(as_ptr<sentil_ring_buffer_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferLen(JNIEnv*, jclass,
                                                                              jlong handle) {
    return static_cast<jlong>(sentil_ring_buffer_len(as_ptr<const sentil_ring_buffer_t>(handle)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferCapacity(JNIEnv*, jclass,
                                                                                   jlong handle) {
    return static_cast<jlong>(
        sentil_ring_buffer_capacity(as_ptr<const sentil_ring_buffer_t>(handle)));
}

JNIEXPORT jboolean JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferIsEmpty(JNIEnv*,
                                                                                     jclass,
                                                                                     jlong handle) {
    return sentil_ring_buffer_is_empty(as_ptr<const sentil_ring_buffer_t>(handle)) ? JNI_TRUE
                                                                                   : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferIsFull(JNIEnv*, jclass,
                                                                                    jlong handle) {
    return sentil_ring_buffer_is_full(as_ptr<const sentil_ring_buffer_t>(handle)) ? JNI_TRUE
                                                                                  : JNI_FALSE;
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferFront(JNIEnv* env,
                                                                                  jclass,
                                                                                  jlong handle) {
    return make_sample(env, sentil_ring_buffer_front(as_ptr<const sentil_ring_buffer_t>(handle)));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferBack(JNIEnv* env, jclass,
                                                                                 jlong handle) {
    return make_sample(env, sentil_ring_buffer_back(as_ptr<const sentil_ring_buffer_t>(handle)));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferGet(JNIEnv* env, jclass,
                                                                                jlong handle,
                                                                                jlong index) {
    return make_sample(env, sentil_ring_buffer_get(as_ptr<const sentil_ring_buffer_t>(handle),
                                                   static_cast<size_t>(index)));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferPopFront(JNIEnv* env,
                                                                                     jclass,
                                                                                     jlong handle) {
    return make_sample(env, sentil_ring_buffer_pop_front(as_ptr<sentil_ring_buffer_t>(handle)));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferPopBack(JNIEnv* env,
                                                                                    jclass,
                                                                                    jlong handle) {
    return make_sample(env, sentil_ring_buffer_pop_back(as_ptr<sentil_ring_buffer_t>(handle)));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferClosestToTime(
    JNIEnv* env, jclass, jlong handle, jdouble time) {
    return make_sample(
        env, sentil_ring_buffer_closest_to_time(as_ptr<const sentil_ring_buffer_t>(handle), time));
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferMean(
    JNIEnv* env, jclass, jlong handle) {
    double out = 0.0;
    bool present = sentil_ring_buffer_mean(as_ptr<const sentil_ring_buffer_t>(handle), &out);
    return optional_double(env, present, out);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferVariance(
    JNIEnv* env, jclass, jlong handle) {
    double out = 0.0;
    bool present = sentil_ring_buffer_variance(as_ptr<const sentil_ring_buffer_t>(handle), &out);
    return optional_double(env, present, out);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferStdDev(
    JNIEnv* env, jclass, jlong handle) {
    double out = 0.0;
    bool present = sentil_ring_buffer_std_dev(as_ptr<const sentil_ring_buffer_t>(handle), &out);
    return optional_double(env, present, out);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferMin(
    JNIEnv* env, jclass, jlong handle) {
    double out = 0.0;
    bool present = sentil_ring_buffer_min(as_ptr<const sentil_ring_buffer_t>(handle), &out);
    return optional_double(env, present, out);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferMax(
    JNIEnv* env, jclass, jlong handle) {
    double out = 0.0;
    bool present = sentil_ring_buffer_max(as_ptr<const sentil_ring_buffer_t>(handle), &out);
    return optional_double(env, present, out);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferRecomputeStatistics(
    JNIEnv*, jclass, jlong handle) {
    sentil_ring_buffer_recompute_statistics(as_ptr<sentil_ring_buffer_t>(handle));
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferAtTime(
    JNIEnv* env, jclass, jlong handle, jdouble time) {
    double out = 0.0;
    bool present = sentil_ring_buffer_at_time(as_ptr<const sentil_ring_buffer_t>(handle), time, &out);
    return optional_double(env, present, out);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferTimeRange(
    JNIEnv* env, jclass, jlong handle) {
    double start = 0.0;
    double end = 0.0;
    bool present =
        sentil_ring_buffer_time_range(as_ptr<const sentil_ring_buffer_t>(handle), &start, &end);
    jdoubleArray result = env->NewDoubleArray(present ? 2 : 0);
    if (present && result != nullptr) {
        double bounds[2] = {start, end};
        env->SetDoubleArrayRegion(result, 0, 2, bounds);
    }
    return result;
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferBetween(
    JNIEnv* env, jclass, jlong handle, jdouble start, jdouble end) {
    size_t count = 0;
    sentil_sample_t* samples =
        sentil_ring_buffer_between(as_ptr<const sentil_ring_buffer_t>(handle), start, end, &count);
    if (samples == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObjectArray(0, cls_sample, nullptr);
    }
    return owned_sample_array(env, samples, count);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_ringBufferDestroy(JNIEnv*, jclass,
                                                                                 jlong handle) {
    sentil_ring_buffer_destroy(as_ptr<sentil_ring_buffer_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_configCreate(JNIEnv* env, jclass) {
    sentil_monitor_config_t* config = sentil_monitor_config_create();
    if (config == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(config);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_configSetTime(JNIEnv* env, jclass,
                                                                             jlong handle,
                                                                             jint mode) {
    sentil_error_t code = sentil_monitor_config_set_time(as_ptr<sentil_monitor_config_t>(handle),
                                                         static_cast<sentil_time_mode_t>(mode));
    failed(env, code);
}

JNIEXPORT jint JNICALL Java_io_github_sedislab_sentil_NativeLib_configTimeMode(JNIEnv*, jclass,
                                                                              jlong handle) {
    return static_cast<jint>(
        sentil_monitor_config_time_mode(as_ptr<const sentil_monitor_config_t>(handle)));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_configDestroy(JNIEnv*, jclass,
                                                                             jlong handle) {
    sentil_monitor_config_destroy(as_ptr<sentil_monitor_config_t>(handle));
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaViolations(
    JNIEnv* env, jclass, jlong formula, jlong trace) {
    size_t count = 0;
    sentil_interval_t* spans = sentil_formula_violations(as_ptr<const sentil_formula_t>(formula),
                                                         as_ptr<const sentil_trace_t>(trace),
                                                         &count);
    if (spans == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObjectArray(0, cls_interval, nullptr);
    }
    return owned_interval_array(env, spans, count);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorCreate(JNIEnv* env, jclass,
                                                                              jlong formula,
                                                                              jlong config) {
    const sentil_monitor_config_t* cfg =
        config == 0 ? nullptr : as_ptr<const sentil_monitor_config_t>(config);
    sentil_monitor_t* monitor = sentil_monitor_create(as_ptr<sentil_formula_t>(formula), cfg);
    if (monitor == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(monitor);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorParse(JNIEnv* env, jclass,
                                                                             jstring formula,
                                                                             jlong config) {
    Utf8 text(env, formula);
    const sentil_monitor_config_t* cfg =
        config == 0 ? nullptr : as_ptr<const sentil_monitor_config_t>(config);
    sentil_monitor_t* monitor = sentil_monitor_parse(text.c(), cfg);
    if (monitor == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(monitor);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorFormula(JNIEnv* env, jclass,
                                                                               jlong handle) {
    sentil_formula_t* formula = sentil_monitor_formula(as_ptr<const sentil_monitor_t>(handle));
    if (formula == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(formula);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorConfig(JNIEnv* env, jclass,
                                                                              jlong handle) {
    sentil_monitor_config_t* config = sentil_monitor_config(as_ptr<const sentil_monitor_t>(handle));
    if (config == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(config);
}

JNIEXPORT jdouble JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorRobustness(
    JNIEnv* env, jclass, jlong handle, jlong trace) {
    double out = 0.0;
    sentil_error_t code = sentil_monitor_robustness(as_ptr<const sentil_monitor_t>(handle),
                                                    as_ptr<const sentil_trace_t>(trace), &out);
    if (failed(env, code)) {
        return 0.0;
    }
    return out;
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorRobustnessSignal(
    JNIEnv* env, jclass, jlong handle, jlong trace) {
    size_t length = 0;
    double* signal = sentil_monitor_robustness_signal(as_ptr<const sentil_monitor_t>(handle),
                                                      as_ptr<const sentil_trace_t>(trace), &length);
    if (signal == nullptr) {
        raise_last(env);
        return nullptr;
    }
    return owned_doubles(env, signal, length);
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorViolations(
    JNIEnv* env, jclass, jlong handle, jlong trace) {
    size_t count = 0;
    sentil_interval_t* spans = sentil_monitor_violations(as_ptr<const sentil_monitor_t>(handle),
                                                         as_ptr<const sentil_trace_t>(trace),
                                                         &count);
    if (spans == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObjectArray(0, cls_interval, nullptr);
    }
    return owned_interval_array(env, spans, count);
}

JNIEXPORT jlongArray JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorSymbolIndex(
    JNIEnv* env, jclass, jlong handle, jstring name) {
    Utf8 variable(env, name);
    size_t index = 0;
    bool found = false;
    sentil_error_t code = sentil_monitor_symbol_index(as_ptr<sentil_monitor_t>(handle),
                                                      variable.c(), &index, &found);
    if (failed(env, code)) {
        return nullptr;
    }
    jlongArray result = env->NewLongArray(found ? 1 : 0);
    if (found && result != nullptr) {
        jlong value = static_cast<jlong>(index);
        env->SetLongArrayRegion(result, 0, 1, &value);
    }
    return result;
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorUpdate(
    JNIEnv* env, jclass, jlong handle, jdouble time, jobjectArray names, jdoubleArray values) {
    StringArray name_guard(env, names);
    DoubleArray value_guard(env, values);
    sentil_robustness_t out;
    sentil_error_t code = sentil_monitor_update(as_ptr<sentil_monitor_t>(handle), time,
                                                name_guard.data(), value_guard.data(),
                                                value_guard.size(), &out);
    if (failed(env, code)) {
        return nullptr;
    }
    return make_robustness(env, out);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorUpdatePacked(
    JNIEnv* env, jclass, jlong handle, jdouble time, jdoubleArray values) {
    DoubleArray value_guard(env, values);
    sentil_robustness_t out;
    sentil_error_t code = sentil_monitor_update_packed(as_ptr<sentil_monitor_t>(handle), time,
                                                       value_guard.data(), value_guard.size(),
                                                       &out);
    if (failed(env, code)) {
        return nullptr;
    }
    return make_robustness(env, out);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorReset(JNIEnv*, jclass,
                                                                            jlong handle) {
    sentil_monitor_reset(as_ptr<sentil_monitor_t>(handle));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorDestroy(JNIEnv*, jclass,
                                                                              jlong handle) {
    sentil_monitor_destroy(as_ptr<sentil_monitor_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_streamMonitorCreate(
    JNIEnv* env, jclass, jstring formula) {
    Utf8 text(env, formula);
    sentil_stream_monitor_t* monitor = sentil_stream_monitor_create(text.c());
    if (monitor == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(monitor);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_streamMonitorFromFormula(
    JNIEnv* env, jclass, jlong formula) {
    sentil_stream_monitor_t* monitor =
        sentil_stream_monitor_from_formula(as_ptr<const sentil_formula_t>(formula));
    if (monitor == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(monitor);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_streamMonitorVariableCount(
    JNIEnv*, jclass, jlong handle) {
    return static_cast<jlong>(
        sentil_stream_monitor_variable_count(as_ptr<const sentil_stream_monitor_t>(handle)));
}

JNIEXPORT jlongArray JNICALL Java_io_github_sedislab_sentil_NativeLib_streamMonitorSymbolIndex(
    JNIEnv* env, jclass, jlong handle, jstring name) {
    Utf8 variable(env, name);
    size_t index = 0;
    bool found = false;
    sentil_error_t code = sentil_stream_monitor_symbol_index(
        as_ptr<const sentil_stream_monitor_t>(handle), variable.c(), &index, &found);
    if (failed(env, code)) {
        return nullptr;
    }
    jlongArray result = env->NewLongArray(found ? 1 : 0);
    if (found && result != nullptr) {
        jlong value = static_cast<jlong>(index);
        env->SetLongArrayRegion(result, 0, 1, &value);
    }
    return result;
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_streamMonitorUpdate(
    JNIEnv* env, jclass, jlong handle, jdouble time, jobjectArray names, jdoubleArray values) {
    StringArray name_guard(env, names);
    DoubleArray value_guard(env, values);
    sentil_robustness_t out;
    sentil_error_t code = sentil_stream_monitor_update(as_ptr<sentil_stream_monitor_t>(handle), time,
                                                       name_guard.data(), value_guard.data(),
                                                       value_guard.size(), &out);
    if (failed(env, code)) {
        return nullptr;
    }
    return make_robustness(env, out);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_streamMonitorUpdatePacked(
    JNIEnv* env, jclass, jlong handle, jdouble time, jdoubleArray values) {
    DoubleArray value_guard(env, values);
    sentil_robustness_t out;
    sentil_error_t code = sentil_stream_monitor_update_packed(
        as_ptr<sentil_stream_monitor_t>(handle), time, value_guard.data(), value_guard.size(), &out);
    if (failed(env, code)) {
        return nullptr;
    }
    return make_robustness(env, out);
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_streamMonitorRun(
    JNIEnv* env, jclass, jlong handle, jlong trace) {
    size_t count = 0;
    sentil_robustness_t* verdicts = sentil_stream_monitor_run(
        as_ptr<sentil_stream_monitor_t>(handle), as_ptr<const sentil_trace_t>(trace), &count);
    if (verdicts == nullptr) {
        raise_last(env);
        return nullptr;
    }
    return owned_robustness_array(env, verdicts, count);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_streamMonitorReset(JNIEnv*, jclass,
                                                                                  jlong handle) {
    sentil_stream_monitor_reset(as_ptr<sentil_stream_monitor_t>(handle));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_streamMonitorDestroy(JNIEnv*, jclass,
                                                                                    jlong handle) {
    sentil_stream_monitor_destroy(as_ptr<sentil_stream_monitor_t>(handle));
}