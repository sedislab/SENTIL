#include "io_github_sedislab_sentil_NativeLib.h"

#include <sentil.h>

#include <cstdint>
#include <string>

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
    return JNI_VERSION_1_8;
}

JNIEXPORT void JNICALL JNI_OnUnload(JavaVM* vm, void*) {
    JNIEnv* env = nullptr;
    if (vm->GetEnv(reinterpret_cast<void**>(&env), JNI_VERSION_1_8) != JNI_OK) {
        return;
    }
    for (jclass* slot : {&cls_base, &cls_parse, &cls_semantic, &cls_evaluation, &cls_string}) {
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