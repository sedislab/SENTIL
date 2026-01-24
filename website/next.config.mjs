import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createMDX } from 'fumadocs-mdx/next';

const withMDX = createMDX();

const dir = path.dirname(fileURLToPath(import.meta.url));
const pageActionsStub = path.resolve(dir, 'src/lib/page-actions-stub.tsx');

const config = {
  output: 'export',
  images: { unoptimized: true },
  reactStrictMode: true,
  experimental: {
    cpus: 2,
  },
  // This hook is why the build passes --webpack; Turbopack ignores it, and the
  // stub it installs is what keeps outside service names out of the bundle.
  webpack: (cfg, { webpack }) => {
    cfg.plugins.push(
      new webpack.NormalModuleReplacementPlugin(/shared\/page-actions\.js$/, (resource) => {
        if ((resource.context || '').includes('fumadocs-ui')) {
          resource.request = pageActionsStub;
        }
      }),
    );
    return cfg;
  },
};

export default withMDX(config);