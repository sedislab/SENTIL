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
    return JNI_VERSION_1_8;
}

JNIEXPORT void JNICALL JNI_OnUnload(JavaVM* vm, void*) {
    JNIEnv* env = nullptr;
    if (vm->GetEnv(reinterpret_cast<void**>(&env), JNI_VERSION_1_8) != JNI_OK) {
        return;
    }
    for (jclass* slot : {&cls_base, &cls_parse, &cls_semantic, &cls_evaluation}) {
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