function h = consume_handles(items)
%CONSUME_HANDLES Surrender the native handles of an array of wrappers.
h = zeros(1, numel(items), 'uint64');
for i = 1:numel(items)
    h(i) = items(i).consume();
end
end