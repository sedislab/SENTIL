function [names, values] = unpack_sample(sample)
%UNPACK_SAMPLE Split a struct or containers.Map sample into name and value arrays.
if isa(sample, 'containers.Map')
    % Method syntax; `values` is already this function's output name.
    names = keys(sample);
    values = cell2mat(sample.values);
elseif isstruct(sample)
    names = fieldnames(sample)';
    values = zeros(1, numel(names));
    for i = 1:numel(names)
        values(i) = sample.(names{i});
    end
else
    error('sentil:sample', 'a sample must be a struct or a containers.Map');
end
end