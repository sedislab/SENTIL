function run_fda_insulin_benchmark()
% RUN_FDA_INSULIN_BENCHMARK PrSTL verification of a closed-loop artificial pancreas.
%
%   Kovatchev et al., J. Diabetes Sci. Tech. 3(1), 2009.

    bdclose('all');
    close all; clc;

    script_root = fileparts(mfilename('fullpath'));
    if isempty(script_root), script_root = pwd; end

    binding_root = fileparts(script_root);
    addpath(fullfile(binding_root, 'blocks'));
    if ~any(strcmp(regexp(path, pathsep, 'split'), binding_root))
        addpath(binding_root);
    end

    banner('SENTIL medical-device benchmark: UVA/Padova insulin pump');

    num_samples = 1000;
    sim_time    = 86400;         % seconds (24 h)
    dt_fixed    = 1;             % fixed-step ode3 size (s)
    slx_model   = 'sentil_fda_insulin_benchmark';

    target_bg   = 120;           % controller setpoint (mg/dL)
    Kp_baseline = 0.050;
    Kp_retuned  = 0.038;

    req_normo = 'always[0, 86400] (BG >= 70.0 and BG <= 180.0)';
    req_hypo  = 'eventually[0, 86400] (BG < 60.0)';

    fprintf('[1/5] Generating 7D LHS parameter space (%d patients)...\n', ...
            num_samples);
    rng(42);
    X = lhsdesign(num_samples, 7);

    patients.SI        = 50  + (250 - 50)  * X(:,1); % insulin sensitivity
    patients.tau_c     = 10  + (60  - 10)  * X(:,2); % gut time constant (min)
    patients.EGP       = 1.0 + (3.0 - 1.0) * X(:,3); % endogenous production
    patients.GU        = 0.5 + (1.5 - 0.5) * X(:,4); % glucose utilization
    patients.GE        = 10  + (50  - 10)  * X(:,5); % gastric emptying (min)
    patients.BW        = 40  + (120 - 40)  * X(:,6); % body weight (kg)
    patients.CGM_sigma = 1.0 + (5.0 - 1.0) * X(:,7); % CGM noise SD (mg/dL)

    meal_times = [8*3600, 12*3600, 18*3600];
    meal_cho   = [60, 80, 70];                % grams of CHO
    pulse_dur  = 300;                         % 5 min, in seconds

    t_meal    = (0:10:sim_time)';
    carb_flux = zeros(size(t_meal));          % g/min
    for k = 1:numel(meal_times)
        inside = t_meal >= meal_times(k) & t_meal < meal_times(k) + pulse_dur;
        carb_flux(inside) = meal_cho(k) / (pulse_dur/60);
    end
    meal_ts = timeseries(carb_flux, t_meal);
    meal_ts.Name = 'meal_ts';

    fprintf('[2/5] Constructing Simulink model: %s.slx ...\n', slx_model);
    build_insulin_model(slx_model, target_bg, req_normo, req_hypo, sim_time, dt_fixed);

    try
        print(['-s' slx_model], '-dpng', '-r200', 'insulin_model_diagram.png');
        fprintf('      diagram -> insulin_model_diagram.png\n');
    catch
        fprintf('      diagram export skipped (headless node)\n');
    end

    fprintf('[3/5] Parallel verification, baseline Kp = %.3f ...\n', Kp_baseline);
    baseline = run_cohort(slx_model, patients, meal_ts, Kp_baseline);

    region = patients.SI > 180 & patients.tau_c < 25;
    nR = sum(region);
    p_violate_base = mean(~baseline.Safe_Normo(region));

    rule('-');
    fprintf(' Problem region (SI > 180, tau_c < 25) : %d traces\n', nR);
    fprintf(' Violation probability (baseline)      : %.3f   [target: 0.043]\n', ...
            p_violate_base);
    rule('-');

    try
        f = figure('Visible', 'off', 'Position', [0 0 800 600]);
        scatter(patients.SI, patients.tau_c, 22, baseline.Safe_Normo, 'filled');
        colormap([0.85 0.15 0.15; 0.15 0.65 0.25]);
        caxis([0 1]);
        xline(180, '--k'); yline(25, '--k');
        xlabel('Insulin sensitivity, SI');
        ylabel('Carbohydrate absorption time, \tau_c (min)');
        title('Baseline verification  (red: PrSTL violation)');
        saveas(f, 'parameter_space_analysis.png');
        close(f);
    catch
    end

    fprintf('[4/5] Parallel verification, retuned Kp = %.3f ...\n', Kp_retuned);
    retuned = run_cohort(slx_model, patients, meal_ts, Kp_retuned);
    p_violate_ret = mean(~retuned.Safe_Normo(region));
    fprintf(' Violation probability (retuned)       : %.3f   [target: 0.011]\n', ...
            p_violate_ret);

    fprintf('[5/5] Writing verification report and trace tables...\n');

    g_normo_base = mean(baseline.Safe_Normo);
    g_hypo_base  = 1 - mean(baseline.Safe_Hypo);
    g_normo_ret  = mean(retuned.Safe_Normo);
    g_hypo_ret   = 1 - mean(retuned.Safe_Hypo);

    banner('PrSTL verification results');
    fprintf(' R1:  P[>= 0.999] ( G[0,86400] 70 <= BG <= 180 )\n');
    fprintf(' R2:  P[<  0.001] ( F[0,86400] BG < 60 )\n\n');

    fprintf(' Baseline  (Kp = %.3f)\n', Kp_baseline);
    fprintf('   Pr[normoglycemia]  : %.4f   pass: %s\n', ...
            g_normo_base, yesno(g_normo_base >= 0.999));
    fprintf('   Pr[severe hypo]    : %.4f   pass: %s\n\n', ...
            g_hypo_base,  yesno(g_hypo_base  <  0.001));

    fprintf(' Retuned   (Kp = %.3f)\n', Kp_retuned);
    fprintf('   Pr[normoglycemia]  : %.4f   pass: %s\n', ...
            g_normo_ret,  yesno(g_normo_ret  >= 0.999));
    fprintf('   Pr[severe hypo]    : %.4f   pass: %s\n', ...
            g_hypo_ret,   yesno(g_hypo_ret   <  0.001));
    banner('');

    writetable(baseline, 'insulin_benchmark_baseline.csv');
    writetable(retuned,  'insulin_benchmark_retuned.csv');
    fprintf('      traces -> insulin_benchmark_{baseline,retuned}.csv\n');

    close_system(slx_model, 0);
end

function results = run_cohort(slx_model, p, meal_ts, Kp)
%RUN_COHORT Execute the patient cohort in parallel via parsim.
    n = numel(p.SI);
    simIn = repmat(Simulink.SimulationInput(slx_model), 1, n);
    for i = 1:n
        simIn(i) = simIn(i) ...
            .setVariable('SI',        p.SI(i)) ...
            .setVariable('tau_c',     p.tau_c(i)) ...
            .setVariable('EGP',       p.EGP(i)) ...
            .setVariable('GU',        p.GU(i)) ...
            .setVariable('GE',        p.GE(i)) ...
            .setVariable('BW',        p.BW(i)) ...
            .setVariable('CGM_sigma', p.CGM_sigma(i)) ...
            .setVariable('Kp',        Kp) ...
            .setVariable('meal_ts',   meal_ts);
    end

    pool = gcp('nocreate');
    if isempty(pool)
        pool = parpool('local');
    end
    if pool.NumWorkers < 2
        warning('run_cohort:serial', ...
            ['Parallel pool has %d worker. Cohort will execute serially.\n' ...
             'Inspect the local cluster profile (NumWorkers) and the SLURM ' ...
             'allocation to recover wall-clock performance.'], ...
            pool.NumWorkers);
    end

    t0 = tic;
    simOut = parsim(simIn, ...
        'ShowProgress', 'on', ...
        'TransferBaseWorkspaceVariables', 'on', ...
        'StopOnError', 'off');
    elapsed = toc(t0);

    failed = arrayfun(@(s) ~isempty(s.ErrorMessage), simOut);
    if all(failed)
        fprintf('\n[FATAL] All %d simulations failed. First error message:\n%s\n', ...
                n, simOut(1).ErrorMessage);
        error('run_cohort:allFailed', 'Every simulation errored out.');
    end
    if any(failed)
        warning('run_cohort:someFailed', ...
            '%d of %d simulations failed and are recorded as violations.', ...
            sum(failed), n);
    end

    fprintf('      completed in %.2f min on %d workers\n', ...
            elapsed/60, pool.NumWorkers);

    safe_normo = false(n, 1);
    safe_hypo  = false(n, 1); % safe means F(BG < 60) did not occur
    for i = 1:n
        if failed(i), continue; end
        rn = simOut(i).get('rob_normo');
        rh = simOut(i).get('rob_hypo');
        safe_normo(i) = rn(end) > 0;
        safe_hypo(i)  = rh(end) <= 0;
    end

    results = table((1:n)', safe_normo, safe_hypo, ...
        'VariableNames', {'Patient_ID', 'Safe_Normo', 'Safe_Hypo'});
end

function banner(msg)
    bar = repmat('=', 1, 66);
    fprintf('%s\n', bar);
    if ~isempty(msg)
        fprintf(' %s\n', msg);
        fprintf('%s\n', bar);
    end
end

function rule(ch)
    fprintf('%s\n', repmat(ch, 1, 66));
end

function s = yesno(x)
    if x, s = 'yes'; else, s = 'NO '; end
end