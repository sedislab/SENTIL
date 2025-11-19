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
jclass cls_confidence = nullptr;
jmethodID ctor_confidence = nullptr;
jclass cls_smc_result = nullptr;
jmethodID ctor_smc_result = nullptr;
jclass cls_distribution = nullptr;
jmethodID ctor_distribution = nullptr;
jclass cls_sprt_result = nullptr;
jmethodID ctor_sprt_result = nullptr;
jclass cls_bayes_result = nullptr;
jmethodID ctor_bayes_result = nullptr;
jclass cls_rare_result = nullptr;
jmethodID ctor_rare_result = nullptr;
jclass cls_synth_result = nullptr;
jmethodID ctor_synth_result = nullptr;
jclass cls_chance_report = nullptr;
jmethodID ctor_chance_report = nullptr;
jclass cls_witness = nullptr;
jmethodID ctor_witness = nullptr;
jclass cls_object = nullptr;

jmethodID mid_bernoulli = nullptr;

jclass cls_linked_map = nullptr;
jmethodID ctor_linked_map = nullptr;
jmethodID mid_map_put = nullptr;
jclass cls_double = nullptr;
jmethodID mid_double_value_of = nullptr;

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

jobject make_confidence(JNIEnv* env, const sentil_confidence_interval_t& c) {
    return env->NewObject(cls_confidence, ctor_confidence, c.lower, c.upper, c.level);
}

jobject make_smc_result(JNIEnv* env, const sentil_smc_result_t& r) {
    jobject interval = make_confidence(env, r.interval);
    jobject result = env->NewObject(cls_smc_result, ctor_smc_result, r.probability, interval,
                                    static_cast<jlong>(r.satisfactions),
                                    static_cast<jlong>(r.samples), r.holds ? JNI_TRUE : JNI_FALSE);
    env->DeleteLocalRef(interval);
    return result;
}

jobject make_distribution(JNIEnv* env, const sentil_robustness_distribution_t& d) {
    return env->NewObject(cls_distribution, ctor_distribution, static_cast<jlong>(d.count), d.mean,
                          d.variance, d.std_dev, d.min, d.max);
}

jobject make_sprt_result(JNIEnv* env, const sentil_sprt_result_t& r) {
    return env->NewObject(cls_sprt_result, ctor_sprt_result, static_cast<jint>(r.verdict),
                          static_cast<jlong>(r.samples), r.log_likelihood);
}

jobject make_bayes_result(JNIEnv* env, const sentil_bayes_result_t& r) {
    return env->NewObject(cls_bayes_result, ctor_bayes_result, static_cast<jint>(r.verdict),
                          static_cast<jlong>(r.samples), r.posterior);
}

jobject make_rare_event_result(JNIEnv* env, const sentil_rare_event_result_t& r) {
    return env->NewObject(cls_rare_result, ctor_rare_result, r.probability, r.violation_probability,
                          r.holds ? JNI_TRUE : JNI_FALSE, static_cast<jlong>(r.simulations));
}

jobject make_chance_report(JNIEnv* env, const sentil_chance_report_t& r) {
    return env->NewObject(cls_chance_report, ctor_chance_report, r.estimate, r.lower_bound,
                          static_cast<jlong>(r.samples), r.holds ? JNI_TRUE : JNI_FALSE);
}

jobject make_witness(JNIEnv* env, sentil_witness_t& w) {
    jdoubleArray input = copy_doubles(env, w.input, w.input_len);
    if (w.input != nullptr) {
        sentil_free_doubles(w.input, w.input_len);
        w.input = nullptr;
    }
    jlong trace = reinterpret_cast<jlong>(w.trace);
    w.trace = nullptr;
    return env->NewObject(cls_witness, ctor_witness, input, w.robustness, trace);
}

void free_witness(sentil_witness_t& w) {
    if (w.input != nullptr) {
        sentil_free_doubles(w.input, w.input_len);
        w.input = nullptr;
    }
    if (w.trace != nullptr) {
        sentil_trace_destroy(w.trace);
        w.trace = nullptr;
    }
}

sentil_sprt_config_t sprt_config(jdouble p0, jdouble p1, jdouble alpha, jdouble beta,
                                 jlong maxSamples, jlong seed) {
    sentil_sprt_config_t config;
    config.p0 = p0;
    config.p1 = p1;
    config.alpha = alpha;
    config.beta = beta;
    config.max_samples = static_cast<uint64_t>(maxSamples);
    config.seed = static_cast<uint64_t>(seed);
    return config;
}

void take_pending(JNIEnv* env, jthrowable& slot) {
    if (slot == nullptr) {
        jthrowable thrown = env->ExceptionOccurred();
        slot = static_cast<jthrowable>(env->NewGlobalRef(thrown));
        env->DeleteLocalRef(thrown);
    }
    env->ExceptionClear();
}

bool rethrow_captured(JNIEnv* env, jthrowable& slot) {
    if (slot == nullptr) {
        return false;
    }
    env->Throw(slot);
    env->DeleteGlobalRef(slot);
    slot = nullptr;
    return true;
}

struct BernoulliBox {
    JNIEnv* env;
    jobject callable;
    jthrowable captured;
};

bool bernoulli_trampoline(void* userdata) {
    BernoulliBox* box = static_cast<BernoulliBox*>(userdata);
    jboolean draw = box->env->CallBooleanMethod(box->callable, mid_bernoulli);
    if (box->env->ExceptionCheck()) {
        take_pending(box->env, box->captured);
        return false;
    }
    return draw == JNI_TRUE;
}

sentil_smc_config_t smc_config(jlong samples, jdouble confidence, jlong seed, jint method) {
    sentil_smc_config_t config;
    config.samples = static_cast<uint64_t>(samples);
    config.confidence = confidence;
    config.seed = static_cast<uint64_t>(seed);
    config.interval_method = static_cast<sentil_interval_method_t>(method);
    return config;
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

jobject owned_named_robustness_map(JNIEnv* env, sentil_named_robustness_t* owned, size_t count) {
    jobject map = env->NewObject(cls_linked_map, ctor_linked_map);
    for (size_t i = 0; i < count; ++i) {
        jstring id = env->NewStringUTF(owned[i].id);
        jobject robustness = make_robustness(env, owned[i].robustness);
        env->CallObjectMethod(map, mid_map_put, id, robustness);
        env->DeleteLocalRef(id);
        env->DeleteLocalRef(robustness);
    }
    sentil_free_named_robustness(owned, count);
    return map;
}

// Build a LinkedHashMap from id to robustness value, in insertion order, and free the
// source. A formula that errored carries NaN, mirroring the core's bank result.
jobject owned_bank_map(JNIEnv* env, sentil_bank_result_t* owned, size_t count) {
    jobject map = env->NewObject(cls_linked_map, ctor_linked_map);
    for (size_t i = 0; i < count; ++i) {
        jstring id = env->NewStringUTF(owned[i].id);
        jobject value = env->CallStaticObjectMethod(cls_double, mid_double_value_of, owned[i].value);
        env->CallObjectMethod(map, mid_map_put, id, value);
        env->DeleteLocalRef(id);
        env->DeleteLocalRef(value);
    }
    sentil_free_bank_results(owned, count);
    return map;
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
            // The JNI spec only guarantees room for 16 local refs.
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
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/ConfidenceInterval", "(DDD)V",
                          &cls_confidence, &ctor_confidence)) {
        return JNI_ERR;
    }
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/SmcResult",
                          "(DLio/github/sedislab/sentil/ConfidenceInterval;JJZ)V", &cls_smc_result,
                          &ctor_smc_result)) {
        return JNI_ERR;
    }
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/RobustnessDistribution", "(JDDDDD)V",
                          &cls_distribution, &ctor_distribution)) {
        return JNI_ERR;
    }
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/SprtResult", "(IJD)V", &cls_sprt_result,
                          &ctor_sprt_result)) {
        return JNI_ERR;
    }
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/BayesResult", "(IJD)V", &cls_bayes_result,
                          &ctor_bayes_result)) {
        return JNI_ERR;
    }
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/RareEventResult", "(DDZJ)V",
                          &cls_rare_result, &ctor_rare_result)) {
        return JNI_ERR;
    }
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/SynthesisResult", "([DDZI)V",
                          &cls_synth_result, &ctor_synth_result)) {
        return JNI_ERR;
    }
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/ChanceReport", "(DDJZ)V",
                          &cls_chance_report, &ctor_chance_report)) {
        return JNI_ERR;
    }
    if (!cache_class_ctor(env, "io/github/sedislab/sentil/Witness", "([DDJ)V", &cls_witness,
                          &ctor_witness)) {
        return JNI_ERR;
    }
    jclass object_local = env->FindClass("java/lang/Object");
    if (object_local == nullptr) {
        return JNI_ERR;
    }
    cls_object = static_cast<jclass>(env->NewGlobalRef(object_local));
    env->DeleteLocalRef(object_local);
    if (!cache_class_ctor(env, "java/util/LinkedHashMap", "()V", &cls_linked_map,
                          &ctor_linked_map)) {
        return JNI_ERR;
    }
    mid_map_put = env->GetMethodID(cls_linked_map, "put",
                                   "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;");
    if (mid_map_put == nullptr) {
        return JNI_ERR;
    }
    jclass double_local = env->FindClass("java/lang/Double");
    if (double_local == nullptr) {
        return JNI_ERR;
    }
    cls_double = static_cast<jclass>(env->NewGlobalRef(double_local));
    env->DeleteLocalRef(double_local);
    mid_double_value_of = env->GetStaticMethodID(cls_double, "valueOf", "(D)Ljava/lang/Double;");
    if (mid_double_value_of == nullptr) {
        return JNI_ERR;
    }
    jclass supplier = env->FindClass("java/util/function/BooleanSupplier");
    if (supplier == nullptr) {
        return JNI_ERR;
    }
    mid_bernoulli = env->GetMethodID(supplier, "getAsBoolean", "()Z");
    env->DeleteLocalRef(supplier);
    if (mid_bernoulli == nullptr) {
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
                         &cls_sample, &cls_robustness, &cls_interval, &cls_confidence,
                         &cls_smc_result, &cls_distribution, &cls_sprt_result, &cls_bayes_result,
                         &cls_rare_result, &cls_synth_result, &cls_chance_report, &cls_witness,
                         &cls_object, &cls_linked_map, &cls_double}) {
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

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorCreate(JNIEnv* env,
                                                                                   jclass) {
    sentil_multi_monitor_t* monitor = sentil_multi_monitor_create();
    if (monitor == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(monitor);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorAdd(
    JNIEnv* env, jclass, jlong handle, jstring id, jstring formula) {
    Utf8 key(env, id);
    Utf8 text(env, formula);
    failed(env, sentil_multi_monitor_add(as_ptr<sentil_multi_monitor_t>(handle), key.c(), text.c()));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorAddFormula(
    JNIEnv* env, jclass, jlong handle, jstring id, jlong formula) {
    Utf8 key(env, id);
    failed(env, sentil_multi_monitor_add_formula(as_ptr<sentil_multi_monitor_t>(handle), key.c(),
                                                 as_ptr<const sentil_formula_t>(formula)));
}

JNIEXPORT jboolean JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorRemove(
    JNIEnv* env, jclass, jlong handle, jstring id) {
    Utf8 key(env, id);
    return sentil_multi_monitor_remove(as_ptr<sentil_multi_monitor_t>(handle), key.c()) ? JNI_TRUE
                                                                                        : JNI_FALSE;
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorReset(JNIEnv*, jclass,
                                                                                 jlong handle) {
    sentil_multi_monitor_reset(as_ptr<sentil_multi_monitor_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorLen(JNIEnv*, jclass,
                                                                                jlong handle) {
    return static_cast<jlong>(
        sentil_multi_monitor_len(as_ptr<const sentil_multi_monitor_t>(handle)));
}

JNIEXPORT jboolean JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorIsEmpty(
    JNIEnv*, jclass, jlong handle) {
    return sentil_multi_monitor_is_empty(as_ptr<const sentil_multi_monitor_t>(handle)) ? JNI_TRUE
                                                                                       : JNI_FALSE;
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorIds(
    JNIEnv* env, jclass, jlong handle) {
    size_t count = 0;
    char** ids = sentil_multi_monitor_ids(as_ptr<const sentil_multi_monitor_t>(handle), &count);
    if (ids == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObjectArray(0, cls_string, nullptr);
    }
    return owned_string_array(env, ids, count);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorUpdate(
    JNIEnv* env, jclass, jlong handle, jdouble time, jobjectArray names, jdoubleArray values) {
    StringArray name_guard(env, names);
    DoubleArray value_guard(env, values);
    size_t count = 0;
    sentil_named_robustness_t* verdicts = sentil_multi_monitor_update(
        as_ptr<sentil_multi_monitor_t>(handle), time, name_guard.data(), value_guard.data(),
        value_guard.size(), &count);
    if (verdicts == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObject(cls_linked_map, ctor_linked_map);
    }
    return owned_named_robustness_map(env, verdicts, count);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorDestroy(JNIEnv*, jclass,
                                                                                   jlong handle) {
    sentil_multi_monitor_destroy(as_ptr<sentil_multi_monitor_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaBankCreate(JNIEnv* env,
                                                                                  jclass) {
    sentil_formula_bank_t* bank = sentil_formula_bank_create();
    if (bank == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(bank);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaBankAdd(
    JNIEnv* env, jclass, jlong handle, jstring id, jstring formula) {
    Utf8 key(env, id);
    Utf8 text(env, formula);
    failed(env, sentil_formula_bank_add(as_ptr<sentil_formula_bank_t>(handle), key.c(), text.c()));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaBankAddFormula(
    JNIEnv* env, jclass, jlong handle, jstring id, jlong formula) {
    Utf8 key(env, id);
    failed(env, sentil_formula_bank_add_formula(as_ptr<sentil_formula_bank_t>(handle), key.c(),
                                                as_ptr<const sentil_formula_t>(formula)));
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaBankIds(
    JNIEnv* env, jclass, jlong handle) {
    size_t count = 0;
    char** ids = sentil_formula_bank_ids(as_ptr<const sentil_formula_bank_t>(handle), &count);
    if (ids == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObjectArray(0, cls_string, nullptr);
    }
    return owned_string_array(env, ids, count);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaBankLen(JNIEnv*, jclass,
                                                                               jlong handle) {
    return static_cast<jlong>(
        sentil_formula_bank_len(as_ptr<const sentil_formula_bank_t>(handle)));
}

JNIEXPORT jboolean JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaBankIsEmpty(
    JNIEnv*, jclass, jlong handle) {
    return sentil_formula_bank_is_empty(as_ptr<const sentil_formula_bank_t>(handle)) ? JNI_TRUE
                                                                                     : JNI_FALSE;
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaBankRobustness(
    JNIEnv* env, jclass, jlong handle, jlong trace) {
    size_t count = 0;
    sentil_bank_result_t* results = sentil_formula_bank_robustness(
        as_ptr<const sentil_formula_bank_t>(handle), as_ptr<const sentil_trace_t>(trace), &count);
    if (results == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObject(cls_linked_map, ctor_linked_map);
    }
    return owned_bank_map(env, results, count);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaBankRobustnessDense(
    JNIEnv* env, jclass, jlong handle, jlong trace) {
    size_t count = 0;
    sentil_bank_result_t* results = sentil_formula_bank_robustness_dense(
        as_ptr<const sentil_formula_bank_t>(handle), as_ptr<const sentil_trace_t>(trace), &count);
    if (results == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObject(cls_linked_map, ctor_linked_map);
    }
    return owned_bank_map(env, results, count);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaBankDestroy(JNIEnv*, jclass,
                                                                                  jlong handle) {
    sentil_formula_bank_destroy(as_ptr<sentil_formula_bank_t>(handle));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_wilsonInterval(
    JNIEnv* env, jclass, jlong successes, jlong trials, jdouble level) {
    return make_confidence(env, sentil_wilson_interval(static_cast<uint64_t>(successes),
                                                       static_cast<uint64_t>(trials), level));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_clopperPearson(
    JNIEnv* env, jclass, jlong successes, jlong trials, jdouble level) {
    return make_confidence(env, sentil_clopper_pearson(static_cast<uint64_t>(successes),
                                                       static_cast<uint64_t>(trials), level));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_jeffreysInterval(
    JNIEnv* env, jclass, jlong successes, jlong trials, jdouble level) {
    return make_confidence(env, sentil_jeffreys_interval(static_cast<uint64_t>(successes),
                                                         static_cast<uint64_t>(trials), level));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_agrestiCoull(
    JNIEnv* env, jclass, jlong successes, jlong trials, jdouble level) {
    return make_confidence(env, sentil_agresti_coull(static_cast<uint64_t>(successes),
                                                     static_cast<uint64_t>(trials), level));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_intervalByMethod(
    JNIEnv* env, jclass, jint method, jlong successes, jlong trials, jdouble level) {
    return make_confidence(env, sentil_interval(static_cast<sentil_interval_method_t>(method),
                                                static_cast<uint64_t>(successes),
                                                static_cast<uint64_t>(trials), level));
}

JNIEXPORT jdouble JNICALL Java_io_github_sedislab_sentil_NativeLib_zScore(JNIEnv*, jclass,
                                                                         jdouble level) {
    return sentil_z_score(level);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_chernoffHoeffdingSamples(
    JNIEnv* env, jclass, jdouble epsilon, jdouble delta) {
    uint64_t out = 0;
    if (failed(env, sentil_chernoff_hoeffding_samples(epsilon, delta, &out))) {
        return 0;
    }
    return static_cast<jlong>(out);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_wilsonSamples(
    JNIEnv* env, jclass, jdouble epsilon, jdouble level) {
    uint64_t out = 0;
    if (failed(env, sentil_wilson_samples(epsilon, level, &out))) {
        return 0;
    }
    return static_cast<jlong>(out);
}

static jlong return_noise(JNIEnv* env, sentil_noise_model_t* model) {
    if (model == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(model);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseDirac(JNIEnv* env, jclass,
                                                                           jdouble value) {
    return return_noise(env, sentil_noise_dirac(value));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseGaussian(
    JNIEnv* env, jclass, jdouble mean, jdouble stdDev) {
    return return_noise(env, sentil_noise_gaussian(mean, stdDev));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseUniform(JNIEnv* env, jclass,
                                                                             jdouble low,
                                                                             jdouble high) {
    return return_noise(env, sentil_noise_uniform(low, high));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseLogNormal(JNIEnv* env, jclass,
                                                                               jdouble mu,
                                                                               jdouble sigma) {
    return return_noise(env, sentil_noise_log_normal(mu, sigma));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseExponential(JNIEnv* env, jclass,
                                                                                 jdouble rate) {
    return return_noise(env, sentil_noise_exponential(rate));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseGamma(JNIEnv* env, jclass,
                                                                           jdouble shape,
                                                                           jdouble scale) {
    return return_noise(env, sentil_noise_gamma(shape, scale));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseBeta(JNIEnv* env, jclass,
                                                                          jdouble alpha,
                                                                          jdouble beta) {
    return return_noise(env, sentil_noise_beta(alpha, beta));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseWeibull(JNIEnv* env, jclass,
                                                                             jdouble shape,
                                                                             jdouble scale) {
    return return_noise(env, sentil_noise_weibull(shape, scale));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseRayleigh(JNIEnv* env, jclass,
                                                                              jdouble scale) {
    return return_noise(env, sentil_noise_rayleigh(scale));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseGumbel(JNIEnv* env, jclass,
                                                                            jdouble location,
                                                                            jdouble scale) {
    return return_noise(env, sentil_noise_gumbel(location, scale));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseCauchy(JNIEnv* env, jclass,
                                                                            jdouble location,
                                                                            jdouble scale) {
    return return_noise(env, sentil_noise_cauchy(location, scale));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseStudentT(
    JNIEnv* env, jclass, jdouble df, jdouble location, jdouble scale) {
    return return_noise(env, sentil_noise_student_t(df, location, scale));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseTruncatedNormal(
    JNIEnv* env, jclass, jdouble mean, jdouble stdDev, jdouble lower, jdouble upper) {
    return return_noise(env, sentil_noise_truncated_normal(mean, stdDev, lower, upper));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noisePoisson(JNIEnv* env, jclass,
                                                                             jdouble rate) {
    return return_noise(env, sentil_noise_poisson(rate));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseBinomial(JNIEnv* env, jclass,
                                                                              jlong n, jdouble p) {
    return return_noise(env, sentil_noise_binomial(static_cast<uint64_t>(n), p));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseBootstrap(
    JNIEnv* env, jclass, jdoubleArray residuals) {
    DoubleArray data(env, residuals);
    return return_noise(env, sentil_noise_bootstrap(data.data(), data.size()));
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseMean(JNIEnv* env, jclass,
                                                                                 jlong handle) {
    double out = 0.0;
    bool present = sentil_noise_mean(as_ptr<const sentil_noise_model_t>(handle), &out);
    return optional_double(env, present, out);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseVariance(
    JNIEnv* env, jclass, jlong handle) {
    double out = 0.0;
    bool present = sentil_noise_variance(as_ptr<const sentil_noise_model_t>(handle), &out);
    return optional_double(env, present, out);
}

JNIEXPORT jstring JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseToJson(JNIEnv* env, jclass,
                                                                              jlong handle) {
    char* json = sentil_noise_to_json(as_ptr<const sentil_noise_model_t>(handle));
    if (json == nullptr) {
        raise_last(env);
        return nullptr;
    }
    return owned_string(env, json);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseFromJson(JNIEnv* env, jclass,
                                                                              jstring json) {
    Utf8 text(env, json);
    return return_noise(env, sentil_noise_from_json(text.c()));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseFromFile(JNIEnv* env, jclass,
                                                                              jstring path) {
    Utf8 location(env, path);
    return return_noise(env, sentil_noise_from_file(location.c()));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseDestroy(JNIEnv*, jclass,
                                                                            jlong handle) {
    sentil_noise_destroy(as_ptr<sentil_noise_model_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseFitGaussian(
    JNIEnv* env, jclass, jdoubleArray samples) {
    DoubleArray data(env, samples);
    return return_noise(env, sentil_noise_fit_gaussian(data.data(), data.size()));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseFitBootstrap(
    JNIEnv* env, jclass, jdoubleArray samples) {
    DoubleArray data(env, samples);
    return return_noise(env, sentil_noise_fit_bootstrap(data.data(), data.size()));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseFitBootstrapReservoir(
    JNIEnv* env, jclass, jdoubleArray samples, jlong maxSamples) {
    DoubleArray data(env, samples);
    return return_noise(env, sentil_noise_fit_bootstrap_reservoir(data.data(), data.size(),
                                                                  static_cast<size_t>(maxSamples)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseFitGaussianMixture(
    JNIEnv* env, jclass, jdoubleArray samples, jlong components, jlong maxIters) {
    DoubleArray data(env, samples);
    return return_noise(env, sentil_noise_fit_gaussian_mixture(data.data(), data.size(),
                                                               static_cast<size_t>(components),
                                                               static_cast<size_t>(maxIters)));
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseResiduals(
    JNIEnv* env, jclass, jdoubleArray groundTruth, jdoubleArray sensor, jint interaction) {
    DoubleArray truth(env, groundTruth);
    DoubleArray reading(env, sensor);
    size_t length = 0;
    double* residuals = sentil_noise_residuals(
        truth.data(), truth.size(), reading.data(), reading.size(),
        static_cast<sentil_noise_interaction_t>(interaction), &length);
    if (residuals == nullptr) {
        raise_last(env);
        return nullptr;
    }
    return owned_doubles(env, residuals, length);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_noiseMixture(JNIEnv* env, jclass,
                                                                             jdoubleArray weights,
                                                                             jlongArray models) {
    DoubleArray weight_guard(env, weights);
    jsize count = env->GetArrayLength(models);
    if (weight_guard.size() != static_cast<size_t>(count)) {
        throw_sentil(env, SENTIL_ERR_INVALID_CONFIG,
                     "a mixture needs one weight per component model");
        return 0;
    }
    jlong* handles = env->GetLongArrayElements(models, nullptr);
    std::vector<sentil_noise_model_t*> components(static_cast<size_t>(count));
    for (jsize i = 0; i < count; ++i) {
        components[static_cast<size_t>(i)] = as_ptr<sentil_noise_model_t>(handles[i]);
    }
    env->ReleaseLongArrayElements(models, handles, JNI_ABORT);
    return return_noise(env, sentil_noise_mixture(weight_guard.data(), components.data(),
                                                  static_cast<size_t>(count)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_liftingCreate(JNIEnv* env, jclass) {
    sentil_lifting_registry_t* registry = sentil_lifting_registry_create();
    if (registry == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(registry);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_liftingRegister(
    JNIEnv* env, jclass, jlong handle, jstring variable, jlong model, jint interaction) {
    Utf8 name(env, variable);
    failed(env, sentil_lifting_registry_register(
                    as_ptr<sentil_lifting_registry_t>(handle), name.c(),
                    as_ptr<sentil_noise_model_t>(model),
                    static_cast<sentil_noise_interaction_t>(interaction)));
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_liftingVariables(
    JNIEnv* env, jclass, jlong handle) {
    size_t count = 0;
    char** variables =
        sentil_lifting_registry_variables(as_ptr<const sentil_lifting_registry_t>(handle), &count);
    if (variables == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObjectArray(0, cls_string, nullptr);
    }
    return owned_string_array(env, variables, count);
}

JNIEXPORT jboolean JNICALL Java_io_github_sedislab_sentil_NativeLib_liftingIsEmpty(JNIEnv*, jclass,
                                                                                  jlong handle) {
    return sentil_lifting_registry_is_empty(as_ptr<const sentil_lifting_registry_t>(handle))
               ? JNI_TRUE
               : JNI_FALSE;
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_liftingLift(JNIEnv* env, jclass,
                                                                            jlong handle,
                                                                            jlong trace,
                                                                            jlong seed) {
    return return_trace(env, sentil_lifting_registry_lift(
                                 as_ptr<const sentil_lifting_registry_t>(handle),
                                 as_ptr<const sentil_trace_t>(trace), static_cast<uint64_t>(seed)));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_liftingDestroy(JNIEnv*, jclass,
                                                                              jlong handle) {
    sentil_lifting_registry_destroy(as_ptr<sentil_lifting_registry_t>(handle));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaCheck(
    JNIEnv* env, jclass, jlong formula, jlong trace, jlong lifting, jlong samples,
    jdouble confidence, jlong seed, jint method) {
    sentil_smc_config_t config = smc_config(samples, confidence, seed, method);
    sentil_smc_result_t out;
    if (failed(env, sentil_formula_check(as_ptr<const sentil_formula_t>(formula),
                                         as_ptr<const sentil_trace_t>(trace),
                                         as_ptr<const sentil_lifting_registry_t>(lifting), &config,
                                         &out))) {
        return nullptr;
    }
    return make_smc_result(env, out);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaCheckConservative(
    JNIEnv* env, jclass, jlong formula, jlong trace, jlong lifting, jlong samples,
    jdouble confidence, jlong seed, jint method) {
    sentil_smc_config_t config = smc_config(samples, confidence, seed, method);
    sentil_smc_result_t out;
    if (failed(env, sentil_formula_check_conservative(
                        as_ptr<const sentil_formula_t>(formula),
                        as_ptr<const sentil_trace_t>(trace),
                        as_ptr<const sentil_lifting_registry_t>(lifting), &config, &out))) {
        return nullptr;
    }
    return make_smc_result(env, out);
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaCheckDistribution(
    JNIEnv* env, jclass, jlong formula, jlong trace, jlong lifting, jlong samples,
    jdouble confidence, jlong seed, jint method) {
    sentil_smc_config_t config = smc_config(samples, confidence, seed, method);
    sentil_smc_result_t result;
    sentil_robustness_distribution_t distribution;
    if (failed(env, sentil_formula_check_distribution(
                        as_ptr<const sentil_formula_t>(formula),
                        as_ptr<const sentil_trace_t>(trace),
                        as_ptr<const sentil_lifting_registry_t>(lifting), &config, &result,
                        &distribution))) {
        return nullptr;
    }
    jobjectArray pair = env->NewObjectArray(2, cls_object, nullptr);
    jobject smc = make_smc_result(env, result);
    jobject dist = make_distribution(env, distribution);
    env->SetObjectArrayElement(pair, 0, smc);
    env->SetObjectArrayElement(pair, 1, dist);
    env->DeleteLocalRef(smc);
    env->DeleteLocalRef(dist);
    return pair;
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorCheck(
    JNIEnv* env, jclass, jlong monitor, jlong trace, jlong lifting) {
    sentil_smc_result_t out;
    if (failed(env, sentil_monitor_check(as_ptr<const sentil_monitor_t>(monitor),
                                         as_ptr<const sentil_trace_t>(trace),
                                         as_ptr<const sentil_lifting_registry_t>(lifting), &out))) {
        return nullptr;
    }
    return make_smc_result(env, out);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaCheckSequential(
    JNIEnv* env, jclass, jlong formula, jlong trace, jlong lifting, jdouble p0, jdouble p1,
    jdouble alpha, jdouble beta, jlong maxSamples, jlong seed) {
    sentil_sprt_config_t config = sprt_config(p0, p1, alpha, beta, maxSamples, seed);
    sentil_sprt_result_t out;
    if (failed(env, sentil_formula_check_sequential(
                        as_ptr<const sentil_formula_t>(formula),
                        as_ptr<const sentil_trace_t>(trace),
                        as_ptr<const sentil_lifting_registry_t>(lifting), &config, &out))) {
        return nullptr;
    }
    return make_sprt_result(env, out);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorCheckSequential(
    JNIEnv* env, jclass, jlong monitor, jlong trace, jlong lifting, jdouble p0, jdouble p1,
    jdouble alpha, jdouble beta, jlong maxSamples, jlong seed) {
    sentil_sprt_config_t config = sprt_config(p0, p1, alpha, beta, maxSamples, seed);
    sentil_sprt_result_t out;
    if (failed(env, sentil_monitor_check_sequential(
                        as_ptr<const sentil_monitor_t>(monitor),
                        as_ptr<const sentil_trace_t>(trace),
                        as_ptr<const sentil_lifting_registry_t>(lifting), &config, &out))) {
        return nullptr;
    }
    return make_sprt_result(env, out);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaCheckBayesian(
    JNIEnv* env, jclass, jlong formula, jlong trace, jlong lifting, jdouble threshold,
    jdouble bayesFactor, jlong maxSamples, jlong seed) {
    sentil_bayes_config_t config;
    config.threshold = threshold;
    config.bayes_factor = bayesFactor;
    config.max_samples = static_cast<uint64_t>(maxSamples);
    config.seed = static_cast<uint64_t>(seed);
    sentil_bayes_result_t out;
    if (failed(env, sentil_formula_check_bayesian(
                        as_ptr<const sentil_formula_t>(formula),
                        as_ptr<const sentil_trace_t>(trace),
                        as_ptr<const sentil_lifting_registry_t>(lifting), &config, &out))) {
        return nullptr;
    }
    return make_bayes_result(env, out);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_streamMonitorWithLifting(
    JNIEnv* env, jclass, jlong formula, jlong lifting, jlong samples, jdouble confidence, jlong seed,
    jint method) {
    sentil_smc_config_t config = smc_config(samples, confidence, seed, method);
    sentil_stream_monitor_t* monitor = sentil_stream_monitor_with_lifting(
        as_ptr<const sentil_formula_t>(formula),
        as_ptr<const sentil_lifting_registry_t>(lifting), &config);
    if (monitor == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(monitor);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_multiMonitorAddProbabilistic(
    JNIEnv* env, jclass, jlong monitor, jstring id, jlong formula, jlong lifting, jlong samples,
    jdouble confidence, jlong seed, jint method) {
    Utf8 key(env, id);
    sentil_smc_config_t config = smc_config(samples, confidence, seed, method);
    failed(env, sentil_multi_monitor_add_probabilistic(
                    as_ptr<sentil_multi_monitor_t>(monitor), key.c(),
                    as_ptr<const sentil_formula_t>(formula),
                    as_ptr<const sentil_lifting_registry_t>(lifting), &config));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_sequentialTest(
    JNIEnv* env, jclass, jdouble p0, jdouble p1, jdouble alpha, jdouble beta, jlong maxSamples,
    jlong seed, jobject draw) {
    sentil_sprt_config_t config = sprt_config(p0, p1, alpha, beta, maxSamples, seed);
    BernoulliBox box{env, env->NewGlobalRef(draw), nullptr};
    sentil_sprt_result_t out;
    sentil_error_t code =
        sentil_sequential_test(&config, bernoulli_trampoline, &box, &out);
    env->DeleteGlobalRef(box.callable);
    if (rethrow_captured(env, box.captured)) {
        return nullptr;
    }
    if (failed(env, code)) {
        return nullptr;
    }
    return make_sprt_result(env, out);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_bayesSequentialTest(
    JNIEnv* env, jclass, jdouble threshold, jdouble bayesFactor, jlong maxSamples, jlong seed,
    jobject draw) {
    sentil_bayes_config_t config;
    config.threshold = threshold;
    config.bayes_factor = bayesFactor;
    config.max_samples = static_cast<uint64_t>(maxSamples);
    config.seed = static_cast<uint64_t>(seed);
    BernoulliBox box{env, env->NewGlobalRef(draw), nullptr};
    sentil_bayes_result_t out;
    sentil_error_t code =
        sentil_bayes_sequential_test(&config, bernoulli_trampoline, &box, &out);
    env->DeleteGlobalRef(box.callable);
    if (rethrow_captured(env, box.captured)) {
        return nullptr;
    }
    if (failed(env, code)) {
        return nullptr;
    }
    return make_bayes_result(env, out);
}

static jlong return_sim_expr(JNIEnv* env, sentil_sim_expr_t* expr) {
    if (expr == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(expr);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simExprPrev(JNIEnv* env, jclass,
                                                                            jlong variable) {
    return return_sim_expr(env, sentil_sim_expr_prev(static_cast<size_t>(variable)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simExprTime(JNIEnv* env, jclass) {
    return return_sim_expr(env, sentil_sim_expr_time());
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simExprConst(JNIEnv* env, jclass,
                                                                             jdouble value) {
    return return_sim_expr(env, sentil_sim_expr_const(value));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simExprNoise(JNIEnv* env, jclass,
                                                                             jlong source) {
    return return_sim_expr(env, sentil_sim_expr_noise(static_cast<size_t>(source)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simExprAdd(JNIEnv* env, jclass,
                                                                           jlong left, jlong right) {
    return return_sim_expr(env, sentil_sim_expr_add(as_ptr<sentil_sim_expr_t>(left),
                                                    as_ptr<sentil_sim_expr_t>(right)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simExprSub(JNIEnv* env, jclass,
                                                                           jlong left, jlong right) {
    return return_sim_expr(env, sentil_sim_expr_sub(as_ptr<sentil_sim_expr_t>(left),
                                                    as_ptr<sentil_sim_expr_t>(right)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simExprMul(JNIEnv* env, jclass,
                                                                           jlong left, jlong right) {
    return return_sim_expr(env, sentil_sim_expr_mul(as_ptr<sentil_sim_expr_t>(left),
                                                    as_ptr<sentil_sim_expr_t>(right)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simExprDiv(JNIEnv* env, jclass,
                                                                           jlong left, jlong right) {
    return return_sim_expr(env, sentil_sim_expr_div(as_ptr<sentil_sim_expr_t>(left),
                                                    as_ptr<sentil_sim_expr_t>(right)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simExprCall(JNIEnv* env, jclass,
                                                                            jstring name,
                                                                            jlongArray args) {
    Utf8 function(env, name);
    jsize count = env->GetArrayLength(args);
    jlong* handles = env->GetLongArrayElements(args, nullptr);
    std::vector<sentil_sim_expr_t*> operands(static_cast<size_t>(count));
    for (jsize i = 0; i < count; ++i) {
        operands[static_cast<size_t>(i)] = as_ptr<sentil_sim_expr_t>(handles[i]);
    }
    env->ReleaseLongArrayElements(args, handles, JNI_ABORT);
    return return_sim_expr(env, sentil_sim_expr_call(function.c(), operands.data(),
                                                     static_cast<size_t>(count)));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_simExprDestroy(JNIEnv*, jclass,
                                                                              jlong handle) {
    sentil_sim_expr_destroy(as_ptr<sentil_sim_expr_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simModelCreate(
    JNIEnv* env, jclass, jobjectArray variables, jdouble dt, jlong horizon, jlongArray init,
    jlongArray advance, jlongArray noise) {
    StringArray names(env, variables);
    jsize initCount = env->GetArrayLength(init);
    jsize advanceCount = env->GetArrayLength(advance);
    jsize noiseCount = env->GetArrayLength(noise);
    jlong* initHandles = env->GetLongArrayElements(init, nullptr);
    jlong* advanceHandles = env->GetLongArrayElements(advance, nullptr);
    jlong* noiseHandles = env->GetLongArrayElements(noise, nullptr);
    std::vector<sentil_sim_expr_t*> initExprs(static_cast<size_t>(initCount));
    std::vector<sentil_sim_expr_t*> advanceExprs(static_cast<size_t>(advanceCount));
    std::vector<sentil_noise_model_t*> noiseModels(static_cast<size_t>(noiseCount));
    for (jsize i = 0; i < initCount; ++i) {
        initExprs[static_cast<size_t>(i)] = as_ptr<sentil_sim_expr_t>(initHandles[i]);
    }
    for (jsize i = 0; i < advanceCount; ++i) {
        advanceExprs[static_cast<size_t>(i)] = as_ptr<sentil_sim_expr_t>(advanceHandles[i]);
    }
    for (jsize i = 0; i < noiseCount; ++i) {
        noiseModels[static_cast<size_t>(i)] = as_ptr<sentil_noise_model_t>(noiseHandles[i]);
    }
    env->ReleaseLongArrayElements(init, initHandles, JNI_ABORT);
    env->ReleaseLongArrayElements(advance, advanceHandles, JNI_ABORT);
    env->ReleaseLongArrayElements(noise, noiseHandles, JNI_ABORT);
    sentil_sim_model_t* model = sentil_sim_model_create(
        names.data(), names.size(), dt, static_cast<size_t>(horizon), initExprs.data(),
        initExprs.size(), advanceExprs.data(), advanceExprs.size(), noiseModels.data(),
        noiseModels.size());
    if (model == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(model);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simModelSimulate(JNIEnv* env, jclass,
                                                                                 jlong handle,
                                                                                 jlong seed) {
    return return_trace(env, sentil_sim_model_simulate(as_ptr<const sentil_sim_model_t>(handle),
                                                       static_cast<uint64_t>(seed)));
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_simModelVariables(
    JNIEnv* env, jclass, jlong handle) {
    size_t count = 0;
    char** variables = sentil_sim_model_variables(as_ptr<const sentil_sim_model_t>(handle), &count);
    if (variables == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObjectArray(0, cls_string, nullptr);
    }
    return owned_string_array(env, variables, count);
}

JNIEXPORT jdouble JNICALL Java_io_github_sedislab_sentil_NativeLib_simModelDt(JNIEnv*, jclass,
                                                                             jlong handle) {
    return sentil_sim_model_dt(as_ptr<const sentil_sim_model_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simModelHorizon(JNIEnv*, jclass,
                                                                                jlong handle) {
    return static_cast<jlong>(sentil_sim_model_horizon(as_ptr<const sentil_sim_model_t>(handle)));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_simModelToStochasticSystem(
    JNIEnv* env, jclass, jlong handle) {
    sentil_stochastic_system_t* system =
        sentil_sim_model_to_stochastic_system(as_ptr<const sentil_sim_model_t>(handle));
    if (system == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(system);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_simModelDestroy(JNIEnv*, jclass,
                                                                               jlong handle) {
    sentil_sim_model_destroy(as_ptr<sentil_sim_model_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_stochasticSystemSimulate(
    JNIEnv* env, jclass, jlong handle, jlong seed) {
    return return_trace(env, sentil_stochastic_system_simulate(
                                 as_ptr<const sentil_stochastic_system_t>(handle),
                                 static_cast<uint64_t>(seed)));
}

JNIEXPORT jobjectArray JNICALL Java_io_github_sedislab_sentil_NativeLib_stochasticSystemVariables(
    JNIEnv* env, jclass, jlong handle) {
    size_t count = 0;
    char** variables =
        sentil_stochastic_system_variables(as_ptr<const sentil_stochastic_system_t>(handle), &count);
    if (variables == nullptr) {
        if (sentil_get_last_error_code() != SENTIL_OK) {
            raise_last(env);
            return nullptr;
        }
        return env->NewObjectArray(0, cls_string, nullptr);
    }
    return owned_string_array(env, variables, count);
}

JNIEXPORT jdouble JNICALL Java_io_github_sedislab_sentil_NativeLib_stochasticSystemDt(JNIEnv*, jclass,
                                                                                     jlong handle) {
    return sentil_stochastic_system_dt(as_ptr<const sentil_stochastic_system_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_stochasticSystemHorizon(
    JNIEnv*, jclass, jlong handle) {
    return static_cast<jlong>(
        sentil_stochastic_system_horizon(as_ptr<const sentil_stochastic_system_t>(handle)));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_stochasticSystemDestroy(
    JNIEnv*, jclass, jlong handle) {
    sentil_stochastic_system_destroy(as_ptr<sentil_stochastic_system_t>(handle));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaCheckRareEvent(
    JNIEnv* env, jclass, jlong formula, jlong system, jlong particles, jdouble margin, jlong seed) {
    sentil_rare_event_config_t config;
    config.particles = static_cast<size_t>(particles);
    config.margin = margin;
    config.seed = static_cast<uint64_t>(seed);
    sentil_rare_event_result_t out;
    if (failed(env, sentil_formula_check_rare_event(
                        as_ptr<const sentil_formula_t>(formula),
                        as_ptr<const sentil_stochastic_system_t>(system), &config, &out))) {
        return nullptr;
    }
    return make_rare_event_result(env, out);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_monitorCheckRare(
    JNIEnv* env, jclass, jlong monitor, jlong system) {
    sentil_rare_event_result_t out;
    if (failed(env, sentil_monitor_check_rare(as_ptr<const sentil_monitor_t>(monitor),
                                              as_ptr<const sentil_stochastic_system_t>(system),
                                              &out))) {
        return nullptr;
    }
    return make_rare_event_result(env, out);
}

JNIEXPORT jdouble JNICALL Java_io_github_sedislab_sentil_NativeLib_softMin(JNIEnv* env, jclass,
                                                                          jdoubleArray values,
                                                                          jdouble temperature) {
    DoubleArray data(env, values);
    return sentil_soft_min(data.data(), data.size(), temperature);
}

JNIEXPORT jdouble JNICALL Java_io_github_sedislab_sentil_NativeLib_softMax(JNIEnv* env, jclass,
                                                                          jdoubleArray values,
                                                                          jdouble temperature) {
    DoubleArray data(env, values);
    return sentil_soft_max(data.data(), data.size(), temperature);
}

JNIEXPORT jdouble JNICALL Java_io_github_sedislab_sentil_NativeLib_formulaSmoothRobustness(
    JNIEnv* env, jclass, jlong formula, jlong trace, jdouble temperature, jint kind) {
    sentil_smooth_config_t config;
    config.temperature = temperature;
    config.kind = static_cast<sentil_soft_kind_t>(kind);
    double out = 0.0;
    if (failed(env, sentil_formula_smooth_robustness(as_ptr<const sentil_formula_t>(formula),
                                                     as_ptr<const sentil_trace_t>(trace), &config,
                                                     &out))) {
        return 0.0;
    }
    return out;
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_boundsCreate(JNIEnv* env, jclass,
                                                                             jdoubleArray lower,
                                                                             jdoubleArray upper) {
    DoubleArray low(env, lower);
    DoubleArray high(env, upper);
    sentil_bounds_t* bounds = sentil_bounds_create(low.data(), high.data(), low.size());
    if (bounds == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(bounds);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_boundsUnbounded(JNIEnv* env, jclass,
                                                                                jlong dimension) {
    sentil_bounds_t* bounds = sentil_bounds_unbounded(static_cast<size_t>(dimension));
    if (bounds == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(bounds);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_boundsDimension(JNIEnv*, jclass,
                                                                                jlong handle) {
    return static_cast<jlong>(sentil_bounds_dimension(as_ptr<const sentil_bounds_t>(handle)));
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_boundsLower(JNIEnv* env,
                                                                                   jclass,
                                                                                   jlong handle) {
    size_t dimension = sentil_bounds_dimension(as_ptr<const sentil_bounds_t>(handle));
    std::vector<double> out(dimension);
    sentil_bounds_lower(as_ptr<const sentil_bounds_t>(handle), out.data());
    return copy_doubles(env, out.data(), dimension);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_boundsUpper(JNIEnv* env,
                                                                                   jclass,
                                                                                   jlong handle) {
    size_t dimension = sentil_bounds_dimension(as_ptr<const sentil_bounds_t>(handle));
    std::vector<double> out(dimension);
    sentil_bounds_upper(as_ptr<const sentil_bounds_t>(handle), out.data());
    return copy_doubles(env, out.data(), dimension);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_boundsClamp(
    JNIEnv* env, jclass, jlong handle, jdoubleArray point) {
    DoubleArray data(env, point);
    std::vector<double> clamped(data.data(), data.data() + data.size());
    sentil_bounds_clamp(as_ptr<const sentil_bounds_t>(handle), clamped.data(), clamped.size());
    return copy_doubles(env, clamped.data(), clamped.size());
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_boundsDestroy(JNIEnv*, jclass,
                                                                             jlong handle) {
    sentil_bounds_destroy(as_ptr<sentil_bounds_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_linearModelCreate(
    JNIEnv* env, jclass, jdoubleArray a, jlong n, jdoubleArray b, jlong bCols, jdoubleArray x0,
    jobjectArray variables, jdouble dt, jlong horizon) {
    DoubleArray matrixA(env, a);
    DoubleArray matrixB(env, b);
    DoubleArray initial(env, x0);
    StringArray names(env, variables);
    sentil_system_model_t* model = sentil_linear_model_create(
        matrixA.data(), static_cast<size_t>(n), matrixB.data(), static_cast<size_t>(bCols),
        initial.data(), names.data(), names.size(), dt, static_cast<size_t>(horizon));
    if (model == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(model);
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_systemModelInputDimension(
    JNIEnv*, jclass, jlong handle) {
    return static_cast<jlong>(
        sentil_system_model_input_dimension(as_ptr<const sentil_system_model_t>(handle)));
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_systemModelDestroy(JNIEnv*, jclass,
                                                                                  jlong handle) {
    sentil_system_model_destroy(as_ptr<sentil_system_model_t>(handle));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_synthesize(
    JNIEnv* env, jclass, jlong model, jlong spec, jlong bounds, jdouble smoothTemperature,
    jint smoothKind, jboolean hasSmooth, jlong maxIters, jint backend, jlong population) {
    const sentil_bounds_t* boundsPtr = bounds == 0 ? nullptr : as_ptr<const sentil_bounds_t>(bounds);
    sentil_smooth_config_t smooth;
    const sentil_smooth_config_t* smoothPtr = nullptr;
    if (hasSmooth == JNI_TRUE) {
        smooth.temperature = smoothTemperature;
        smooth.kind = static_cast<sentil_soft_kind_t>(smoothKind);
        smoothPtr = &smooth;
    }
    sentil_synthesis_result_t out = {nullptr, 0, 0.0, false, SENTIL_BACKEND_AUTO};
    sentil_error_t code = sentil_synthesize(
        as_ptr<const sentil_system_model_t>(model), as_ptr<const sentil_formula_t>(spec), boundsPtr,
        smoothPtr, static_cast<size_t>(maxIters), static_cast<sentil_backend_t>(backend),
        static_cast<size_t>(population), &out);
    jdoubleArray input = nullptr;
    if (out.input != nullptr) {
        input = copy_doubles(env, out.input, out.input_len);
        sentil_free_doubles(out.input, out.input_len);
    }
    if (failed(env, code)) {
        return nullptr;
    }
    return env->NewObject(cls_synth_result, ctor_synth_result, input, out.robustness,
                          out.holds ? JNI_TRUE : JNI_FALSE, static_cast<jint>(out.backend));
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_solveQp(
    JNIEnv* env, jclass, jdoubleArray p, jlong n, jdoubleArray q, jdoubleArray g, jlong m,
    jdoubleArray h, jlong maxIters) {
    DoubleArray matrixP(env, p);
    DoubleArray vectorQ(env, q);
    DoubleArray matrixG(env, g);
    DoubleArray vectorH(env, h);
    std::vector<double> out(static_cast<size_t>(n));
    if (failed(env, sentil_solve_qp(matrixP.data(), static_cast<size_t>(n), vectorQ.data(),
                                    matrixG.data(), static_cast<size_t>(m), vectorH.data(),
                                    static_cast<size_t>(maxIters), out.data()))) {
        return nullptr;
    }
    return copy_doubles(env, out.data(), out.size());
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_solveSpd(
    JNIEnv* env, jclass, jdoubleArray matrix, jlong n, jdoubleArray rhs) {
    DoubleArray matrixA(env, matrix);
    DoubleArray vectorB(env, rhs);
    std::vector<double> out(static_cast<size_t>(n));
    if (failed(env, sentil_solve_spd(matrixA.data(), static_cast<size_t>(n), vectorB.data(),
                                     out.data()))) {
        return nullptr;
    }
    return copy_doubles(env, out.data(), out.size());
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_symmetricEigen(
    JNIEnv* env, jclass, jdoubleArray matrix, jlong n) {
    DoubleArray matrixA(env, matrix);
    size_t dim = static_cast<size_t>(n);
    std::vector<double> values(dim);
    std::vector<double> vectors(dim * dim);
    if (failed(env, sentil_symmetric_eigen(matrixA.data(), dim, values.data(), vectors.data()))) {
        return nullptr;
    }
    std::vector<double> packed(dim + dim * dim);
    for (size_t i = 0; i < dim; ++i) {
        packed[i] = values[i];
    }
    for (size_t i = 0; i < dim * dim; ++i) {
        packed[dim + i] = vectors[i];
    }
    return copy_doubles(env, packed.data(), packed.size());
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_safetyFilterCreate(JNIEnv* env,
                                                                                   jclass,
                                                                                   jlong bounds) {
    sentil_safety_filter_t* filter = sentil_safety_filter_create(as_ptr<sentil_bounds_t>(bounds));
    if (filter == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(filter);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_safetyFilterFilter(
    JNIEnv* env, jclass, jlong filter, jdoubleArray nominal, jdoubleArray barrierA,
    jdoubleArray barrierB, jlong m) {
    DoubleArray nominalInput(env, nominal);
    DoubleArray a(env, barrierA);
    DoubleArray b(env, barrierB);
    std::vector<double> out(nominalInput.size());
    if (failed(env, sentil_safety_filter_filter(as_ptr<const sentil_safety_filter_t>(filter),
                                                nominalInput.data(), nominalInput.size(), a.data(),
                                                b.data(), static_cast<size_t>(m), out.data()))) {
        return nullptr;
    }
    return copy_doubles(env, out.data(), out.size());
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_safetyFilterDestroy(JNIEnv*, jclass,
                                                                                   jlong handle) {
    sentil_safety_filter_destroy(as_ptr<sentil_safety_filter_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_chanceConstraintCreate(
    JNIEnv* env, jclass, jlong spec, jdouble probability, jdouble confidence, jdouble tightening) {
    sentil_chance_constraint_t* constraint = sentil_chance_constraint_create(
        as_ptr<sentil_formula_t>(spec), probability, confidence, tightening);
    if (constraint == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(constraint);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_chanceConstraintValidate(
    JNIEnv* env, jclass, jlong constraint, jlong system, jlong samples, jlong seed) {
    sentil_chance_report_t out;
    if (failed(env, sentil_chance_constraint_validate(
                        as_ptr<const sentil_chance_constraint_t>(constraint),
                        as_ptr<const sentil_stochastic_system_t>(system),
                        static_cast<uint64_t>(samples), static_cast<uint64_t>(seed), &out))) {
        return nullptr;
    }
    return make_chance_report(env, out);
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_chanceConstraintDestroy(
    JNIEnv*, jclass, jlong handle) {
    sentil_chance_constraint_destroy(as_ptr<sentil_chance_constraint_t>(handle));
}

JNIEXPORT jlong JNICALL Java_io_github_sedislab_sentil_NativeLib_controllerCreate(
    JNIEnv* env, jclass, jlong model, jlong spec, jlong inputWidth, jlong budgetNs, jlong bounds,
    jdouble smoothTemperature, jint smoothKind, jboolean hasSmooth) {
    const sentil_bounds_t* boundsPtr = bounds == 0 ? nullptr : as_ptr<const sentil_bounds_t>(bounds);
    sentil_smooth_config_t smooth;
    const sentil_smooth_config_t* smoothPtr = nullptr;
    if (hasSmooth == JNI_TRUE) {
        smooth.temperature = smoothTemperature;
        smooth.kind = static_cast<sentil_soft_kind_t>(smoothKind);
        smoothPtr = &smooth;
    }
    sentil_controller_t* controller = sentil_controller_create(
        as_ptr<sentil_system_model_t>(model), as_ptr<sentil_formula_t>(spec),
        static_cast<size_t>(inputWidth), static_cast<uint64_t>(budgetNs), boundsPtr, smoothPtr);
    if (controller == nullptr) {
        raise_last(env);
        return 0;
    }
    return reinterpret_cast<jlong>(controller);
}

JNIEXPORT jdoubleArray JNICALL Java_io_github_sedislab_sentil_NativeLib_controllerControl(
    JNIEnv* env, jclass, jlong controller, jdoubleArray state, jlong inputWidth) {
    DoubleArray current(env, state);
    std::vector<double> out(static_cast<size_t>(inputWidth));
    if (failed(env, sentil_controller_control(as_ptr<sentil_controller_t>(controller),
                                              current.data(), current.size(), out.data()))) {
        return nullptr;
    }
    return copy_doubles(env, out.data(), out.size());
}

JNIEXPORT void JNICALL Java_io_github_sedislab_sentil_NativeLib_controllerDestroy(JNIEnv*, jclass,
                                                                                 jlong handle) {
    sentil_controller_destroy(as_ptr<sentil_controller_t>(handle));
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_findCounterexample(
    JNIEnv* env, jclass, jlong formula, jlong model, jlong bounds, jlong maxIters,
    jdouble smoothTemperature, jint smoothKind, jboolean hasSmooth) {
    sentil_smooth_config_t smooth;
    const sentil_smooth_config_t* smoothPtr = nullptr;
    if (hasSmooth == JNI_TRUE) {
        smooth.temperature = smoothTemperature;
        smooth.kind = static_cast<sentil_soft_kind_t>(smoothKind);
        smoothPtr = &smooth;
    }
    sentil_witness_t out = {nullptr, 0, 0.0, nullptr};
    sentil_error_t code = sentil_formula_find_counterexample(
        as_ptr<const sentil_formula_t>(formula), as_ptr<const sentil_system_model_t>(model),
        as_ptr<const sentil_bounds_t>(bounds), static_cast<size_t>(maxIters), smoothPtr, &out);
    if (failed(env, code)) {
        free_witness(out);
        return nullptr;
    }
    return make_witness(env, out);
}

JNIEXPORT jobject JNICALL Java_io_github_sedislab_sentil_NativeLib_falsify(
    JNIEnv* env, jclass, jlong formula, jlong model, jlong bounds, jlong population,
    jlong maxGenerations, jdouble initialStep, jdouble tolStep, jlong seed, jlong restarts) {
    sentil_cma_config_t config;
    config.population = static_cast<size_t>(population);
    config.max_generations = static_cast<size_t>(maxGenerations);
    config.initial_step = initialStep;
    config.tol_step = tolStep;
    config.seed = static_cast<uint64_t>(seed);
    sentil_witness_t out = {nullptr, 0, 0.0, nullptr};
    sentil_error_t code = sentil_formula_falsify(
        as_ptr<const sentil_formula_t>(formula), as_ptr<const sentil_system_model_t>(model),
        as_ptr<const sentil_bounds_t>(bounds), config, static_cast<size_t>(restarts), &out);
    if (failed(env, code)) {
        free_witness(out);
        return nullptr;
    }
    return make_witness(env, out);
}