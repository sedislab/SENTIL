classdef Numerics
    %NUMERICS The convex solvers behind synthesis.

    methods (Static)
        function u = solve_qp(P, q, G, h, maxIters)
            %SOLVE_QP Minimize 1/2 u'Pu + q'u subject to Gu <= h.
            if nargin < 5, maxIters = 1000; end
            u = sentil_mex('solve_qp', P.', size(P, 1), double(q(:).'), G.', size(G, 1), ...
                double(h(:).'), double(maxIters));
        end

        function x = solve_spd(A, b)
            %SOLVE_SPD Solve Ax = b for a symmetric positive-definite A.
            x = sentil_mex('solve_spd', A.', size(A, 1), double(b(:).'));
        end

        function e = symmetric_eigen(M)
            %SYMMETRIC_EIGEN The eigendecomposition of a symmetric matrix, returning a
            %   struct with the eigenvalues and the eigenvectors as columns.
            n = size(M, 1);
            out = sentil_mex('symmetric_eigen', M.', n);
            e = struct('values', out.values, 'vectors', reshape(out.vectors, n, n).');
        end
    end
end