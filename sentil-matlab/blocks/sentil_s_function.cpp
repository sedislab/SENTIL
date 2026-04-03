/* A Simulink Level-2 S-Function that runs the SENTIL streaming monitor on a signal. */

#define S_FUNCTION_NAME sentil_s_function
#define S_FUNCTION_LEVEL 2

#include "simstruc.h"
#include "sentil.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define P_FORMULA 0
#define P_VAR_NAMES 1
#define P_MODE 2
#define P_SAMPLES 3
#define P_NOISE_VAR 4
#define P_NOISE_FILE 5
#define NUM_PARAMS 6

/* Simulink stores only the pointer handed to ssSetErrorStatus. */
static char g_error_buffer[4096];

static void set_rich_error(SimStruct* S, const char* context) {
    size_t needed = sentil_get_last_error_message(NULL, 0);
    if (needed > 0) {
        char* message = (char*)malloc(needed);
        sentil_get_last_error_message(message, needed);
        snprintf(g_error_buffer, sizeof(g_error_buffer), "%s: %s", context, message);
        free(message);
    } else {
        snprintf(g_error_buffer, sizeof(g_error_buffer), "%s", context);
    }
    ssSetErrorStatus(S, g_error_buffer);
}

struct VarNames {
    char** names;
    int_T count;
};

struct MonitorContext {
    sentil_stream_monitor_t* handle;
    sentil_lifting_registry_t* lifter;
    int mode;
    size_t total_symbols;
    size_t* port_to_symbol_map;
    double* packed_buffer;
};

static void freeVarNames(VarNames* vars) {
    if (vars) {
        for (int_T i = 0; i < vars->count; ++i) {
            free(vars->names[i]);
        }
        free(vars->names);
        free(vars);
    }
}

static VarNames* parseVarNames(const mxArray* arr) {
    if (!arr || !mxIsChar(arr)) {
        return NULL;
    }
    char* str = mxArrayToString(arr);
    if (!str) {
        return NULL;
    }
    VarNames* vars = (VarNames*)malloc(sizeof(VarNames));
    vars->count = 0;
    vars->names = NULL;
    char* token = strtok(str, ", ");
    while (token) {
        vars->count++;
        vars->names = (char**)realloc(vars->names, vars->count * sizeof(char*));
        vars->names[vars->count - 1] = strdup(token);
        token = strtok(NULL, ", ");
    }
    mxFree(str);
    return vars;
}

static double safeGetScalar(const mxArray* arr, double def) {
    if (!arr || !mxIsNumeric(arr) || mxIsEmpty(arr)) {
        return def;
    }
    return mxGetScalar(arr);
}

#define MDL_CHECK_PARAMETERS
#if defined(MDL_CHECK_PARAMETERS) && defined(MATLAB_MEX_FILE)
static void mdlCheckParameters(SimStruct* S) {
    const mxArray* formula = ssGetSFcnParam(S, P_FORMULA);
    if (!formula || !mxIsChar(formula) || mxIsEmpty(formula)) {
        ssSetErrorStatus(S, "SENTIL: the formula must be a non-empty string");
        return;
    }
    const mxArray* vars = ssGetSFcnParam(S, P_VAR_NAMES);
    if (!vars || !mxIsChar(vars) || mxIsEmpty(vars)) {
        ssSetErrorStatus(S, "SENTIL: the variable names must be a non-empty string");
        return;
    }
    char* text = mxArrayToString(formula);
    sentil_formula_t* parsed = sentil_formula_parse(text);
    mxFree(text);
    if (!parsed) {
        set_rich_error(S, "SENTIL: the formula does not parse");
        return;
    }
    sentil_formula_destroy(parsed);
}
#endif

static void mdlInitializeSizes(SimStruct* S) {
    ssSetNumSFcnParams(S, NUM_PARAMS);
#if defined(MATLAB_MEX_FILE)
    if (ssGetNumSFcnParams(S) == ssGetSFcnParamsCount(S)) {
        mdlCheckParameters(S);
        if (ssGetErrorStatus(S) != NULL) {
            return;
        }
    } else {
        return;
    }
#endif

    int in_width = 1;
    VarNames* vars = parseVarNames(ssGetSFcnParam(S, P_VAR_NAMES));
    if (vars) {
        in_width = vars->count;
        freeVarNames(vars);
    }

    int mode = (int)safeGetScalar(ssGetSFcnParam(S, P_MODE), 0.0);
    int out_width = (mode == 0) ? 1 : 3;

    ssSetNumContStates(S, 0);
    ssSetNumDiscStates(S, 0);

    if (!ssSetNumInputPorts(S, 1)) {
        return;
    }
    ssSetInputPortWidth(S, 0, in_width);
    ssSetInputPortDirectFeedThrough(S, 0, 1);
    ssSetInputPortDataType(S, 0, SS_DOUBLE);
    ssSetInputPortRequiredContiguous(S, 0, 1);

    if (!ssSetNumOutputPorts(S, 1)) {
        return;
    }
    ssSetOutputPortWidth(S, 0, out_width);
    ssSetOutputPortDataType(S, 0, SS_DOUBLE);

    ssSetNumSampleTimes(S, 1);
    ssSetNumRWork(S, 0);
    ssSetNumIWork(S, 0);
    ssSetNumPWork(S, 2);
    ssSetNumModes(S, 0);
    ssSetNumNonsampledZCs(S, 0);

    ssSetSimStateCompliance(S, USE_DEFAULT_SIM_STATE);
    ssSetOptions(S, SS_OPTION_EXCEPTION_FREE_CODE);
}

static void mdlInitializeSampleTimes(SimStruct* S) {
    ssSetSampleTime(S, 0, INHERITED_SAMPLE_TIME);
    ssSetOffsetTime(S, 0, 0.0);
    ssSetModelReferenceSampleTimeDefaultInheritance(S);
}

#define MDL_START
static void mdlStart(SimStruct* S) {
    ssGetPWork(S)[0] = NULL;
    ssGetPWork(S)[1] = NULL;

    const mxArray* formula_arr = ssGetSFcnParam(S, P_FORMULA);
    const mxArray* vars_arr = ssGetSFcnParam(S, P_VAR_NAMES);
    const mxArray* noise_arr = ssGetSFcnParam(S, P_NOISE_FILE);
    if (!formula_arr || !mxIsChar(formula_arr) || !vars_arr || !mxIsChar(vars_arr)) {
        ssSetErrorStatus(S, "SENTIL: the formula and variable names must be strings");
        return;
    }

    char* formula = mxArrayToString(formula_arr);
    char* noise_file = (noise_arr && mxIsChar(noise_arr)) ? mxArrayToString(noise_arr) : NULL;
    int mode = (int)safeGetScalar(ssGetSFcnParam(S, P_MODE), 0.0);
    double noise_var = safeGetScalar(ssGetSFcnParam(S, P_NOISE_VAR), 0.0);
    uint64_t samples = (uint64_t)safeGetScalar(ssGetSFcnParam(S, P_SAMPLES), 10000.0);

    VarNames* vars = parseVarNames(vars_arr);
    if (!vars || vars->count != ssGetInputPortWidth(S, 0)) {
        if (vars) {
            freeVarNames(vars);
        }
        if (noise_file) {
            mxFree(noise_file);
        }
        mxFree(formula);
        ssSetErrorStatus(S, "SENTIL: the variable count must match the input port width");
        return;
    }

    MonitorContext* ctx = (MonitorContext*)malloc(sizeof(MonitorContext));
    ctx->mode = mode;
    ctx->lifter = NULL;
    ctx->handle = NULL;
    ctx->port_to_symbol_map = NULL;
    ctx->packed_buffer = NULL;

    if (mode > 0) {
        ctx->lifter = sentil_lifting_registry_create();
        int has_file = (noise_file && strlen(noise_file) > 0);
        for (int i = 0; i < vars->count; ++i) {
            sentil_noise_model_t* model = NULL;
            if (has_file) {
                model = sentil_noise_from_file(noise_file);
            } else if (noise_var > 0.0) {
                model = sentil_noise_gaussian(0.0, sqrt(noise_var));
            }
            if (!model) {
                sentil_lifting_registry_destroy(ctx->lifter);
                free(ctx);
                freeVarNames(vars);
                if (noise_file) {
                    mxFree(noise_file);
                }
                mxFree(formula);
                set_rich_error(S, "SENTIL: could not build a noise model for probabilistic mode");
                return;
            }
            if (sentil_lifting_registry_register(ctx->lifter, vars->names[i], model,
                                                 SENTIL_NOISE_ADDITIVE) != SENTIL_OK) {
                mxFree(formula);
                set_rich_error(S, "SENTIL: could not register the noise model for a signal");
                return;
            }
        }
        sentil_formula_t* parsed = sentil_formula_parse(formula);
        if (parsed) {
            sentil_smc_config_t config = sentil_smc_config_default();
            config.samples = samples;
            ctx->handle = sentil_stream_monitor_with_lifting(parsed, ctx->lifter, &config);
            sentil_formula_destroy(parsed);
        }
    } else {
        ctx->handle = sentil_stream_monitor_create(formula);
    }

    mxFree(formula);
    if (noise_file) {
        mxFree(noise_file);
    }

    if (!ctx->handle) {
        if (ctx->lifter) {
            sentil_lifting_registry_destroy(ctx->lifter);
        }
        free(ctx);
        freeVarNames(vars);
        set_rich_error(S, "SENTIL: the monitor could not be created");
        return;
    }

    ctx->port_to_symbol_map = (size_t*)malloc(vars->count * sizeof(size_t));
    int mapping_failed = 0;
    for (int i = 0; i < vars->count; ++i) {
        size_t idx = 0;
        bool found = false;
        if (sentil_stream_monitor_symbol_index(ctx->handle, vars->names[i], &idx, &found) !=
                SENTIL_OK ||
            !found) {
            mapping_failed = 1;
            break;
        }
        ctx->port_to_symbol_map[i] = idx;
    }

    if (mapping_failed) {
        sentil_stream_monitor_destroy(ctx->handle);
        if (ctx->lifter) {
            sentil_lifting_registry_destroy(ctx->lifter);
        }
        free(ctx->port_to_symbol_map);
        free(ctx);
        freeVarNames(vars);
        ssSetErrorStatus(S, "SENTIL: a variable name is not used by the formula");
        return;
    }

    ctx->total_symbols = sentil_stream_monitor_variable_count(ctx->handle);
    ctx->packed_buffer = (double*)calloc(ctx->total_symbols, sizeof(double));

    ssGetPWork(S)[0] = (void*)ctx;
    ssGetPWork(S)[1] = (void*)vars;
}

static void mdlOutputs(SimStruct* S, int_T tid) {
    (void)tid;
    MonitorContext* ctx = (MonitorContext*)ssGetPWork(S)[0];
    VarNames* vars = (VarNames*)ssGetPWork(S)[1];
    if (!ctx || !vars) {
        return;
    }

    int_T width = ssGetInputPortWidth(S, 0);
    const real_T* u = ssGetInputPortRealSignal(S, 0);
    if (!u || width <= 0 || width != vars->count) {
        return;
    }

    for (int_T i = 0; i < width; ++i) {
        ctx->packed_buffer[ctx->port_to_symbol_map[i]] = u[i];
    }

    sentil_robustness_t rob;
    sentil_error_t err = sentil_stream_monitor_update_packed(ctx->handle, ssGetT(S),
                                                             ctx->packed_buffer,
                                                             ctx->total_symbols, &rob);

    real_T* y = ssGetOutputPortRealSignal(S, 0);
    int out_width = ssGetOutputPortWidth(S, 0);
    if (!y || out_width <= 0) {
        return;
    }

    if (err == SENTIL_OK) {
        if (out_width >= 1) {
            y[0] = rob.value;
        }
        if (out_width >= 2) {
            y[1] = rob.lower;
        }
        if (out_width >= 3) {
            y[2] = rob.upper;
        }
    } else {
        set_rich_error(S, "SENTIL: the monitor update failed");
    }
}

static void mdlTerminate(SimStruct* S) {
    void** pwork = ssGetPWork(S);
    if (!pwork) {
        return;
    }
    MonitorContext* ctx = (MonitorContext*)pwork[0];
    if (ctx) {
        if (ctx->handle) {
            sentil_stream_monitor_destroy(ctx->handle);
        }
        if (ctx->lifter) {
            sentil_lifting_registry_destroy(ctx->lifter);
        }
        free(ctx->port_to_symbol_map);
        free(ctx->packed_buffer);
        free(ctx);
        pwork[0] = NULL;
    }
    VarNames* vars = (VarNames*)pwork[1];
    if (vars) {
        freeVarNames(vars);
        pwork[1] = NULL;
    }
}

#ifdef MATLAB_MEX_FILE
#include "simulink.c"
#else
#include "cg_sfun.h"
#endif