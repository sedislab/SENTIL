import { createMDX } from 'fumadocs-mdx/next';

const withMDX = createMDX();

const config = {
  output: 'export',
  images: { unoptimized: true },
  reactStrictMode: true,
  experimental: {
    cpus: 2,
  },
};

export default withMDX(config);