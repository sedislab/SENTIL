classdef SpecBuilder < handle
    %SPECBUILDER The specifications-library loader.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods (Static)
        function names = available()
            %AVAILABLE The names of every embedded specification.
            names = sentil_mex('spec_registry_available');
        end

        function b = from_file(path)
            %FROM_FILE A builder loaded from a spec template file.
            b = sentil.SpecBuilder(uint64(sentil_mex('spec_builder_from_file', char(path))));
        end
    end

    methods
        function obj = SpecBuilder(name)
            %SPECBUILDER A builder for the named spec from the embedded registry.
            if isa(name, 'uint64') && isscalar(name)
                obj.Handle = name;
            else
                obj.Handle = sentil_mex('spec_builder_create', char(name));
            end
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('spec_builder_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function b = with_variant(obj, variant)
            %WITH_VARIANT Select a named variant, consuming this builder.
            b = sentil.SpecBuilder(uint64(sentil_mex('spec_builder_with_variant', obj.consume(), ...
                char(variant))));
        end

        function b = with_param(obj, name, value)
            %WITH_PARAM Override a parameter, consuming this builder.
            b = sentil.SpecBuilder(uint64(sentil_mex('spec_builder_with_param', obj.consume(), ...
                char(name), double(value))));
        end

        function v = available_variants(obj)
            %AVAILABLE_VARIANTS The variant names the spec offers.
            obj.assertOpen();
            v = sentil_mex('spec_builder_available_variants', obj.Handle);
        end

        function s = build_deterministic(obj)
            %BUILD_DETERMINISTIC The deterministic formula text, parameters filled in.
            obj.assertOpen();
            s = sentil_mex('spec_builder_build_deterministic', obj.Handle);
        end

        function s = build_probabilistic(obj)
            %BUILD_PROBABILISTIC The probabilistic formula text, parameters filled in.
            obj.assertOpen();
            s = sentil_mex('spec_builder_build_probabilistic', obj.Handle);
        end

        function f = build_formula(obj)
            %BUILD_FORMULA The deterministic formula as a sentil.Formula.
            obj.assertOpen();
            f = sentil.Formula(sentil_mex('spec_builder_build_formula', obj.Handle));
        end

        function f = build_probabilistic_formula(obj)
            %BUILD_PROBABILISTIC_FORMULA The probabilistic formula as a sentil.Formula.
            obj.assertOpen();
            f = sentil.Formula(sentil_mex('spec_builder_build_probabilistic_formula', obj.Handle));
        end

        function reg = build_lifting_registry(obj)
            %BUILD_LIFTING_REGISTRY A registry from the spec's resolved noise models.
            obj.assertOpen();
            reg = sentil.LiftingRegistry(uint64(sentil_mex('spec_builder_build_lifting_registry', ...
                obj.Handle)));
        end

        function s = parameters_json(obj)
            %PARAMETERS_JSON The resolved parameters as a JSON string.
            obj.assertOpen();
            s = sentil_mex('spec_builder_parameters_json', obj.Handle);
        end

        function m = into_monitor(obj)
            %INTO_MONITOR A monitor with the spec's recommended settings, consuming this builder.
            m = sentil.Monitor(uint64(sentil_mex('spec_builder_into_monitor', obj.consume())));
        end

        function s = smc_settings(obj)
            %SMC_SETTINGS The recommended SMC settings, or [] if none.
            obj.assertOpen();
            s = sentil_mex('spec_builder_smc_settings', obj.Handle);
        end

        function s = sprt_settings(obj)
            %SPRT_SETTINGS The recommended SPRT settings, or [] if none.
            obj.assertOpen();
            s = sentil_mex('spec_builder_sprt_settings', obj.Handle);
        end

        function s = ams_settings(obj)
            %AMS_SETTINGS The recommended rare-event settings, or [] if none.
            obj.assertOpen();
            s = sentil_mex('spec_builder_ams_settings', obj.Handle);
        end
    end

    methods (Hidden)
        function h = consume(obj)
            obj.assertOpen();
            h = obj.Handle;
            obj.Handle = uint64(0);
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this spec builder has been consumed or closed');
            end
        end
    end
end