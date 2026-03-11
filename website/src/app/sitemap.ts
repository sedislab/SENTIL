export const dynamic = 'force-static';

import type { MetadataRoute } from 'next';
import { source } from '@/lib/source';

const SITE = 'https://sentil.pages.dev';

export default function sitemap(): MetadataRoute.Sitemap {
  const pages = source.getPages().map((page) => ({
    url: `${SITE}${page.url}`,
    changeFrequency: 'monthly' as const,
    priority: page.url === '/docs' ? 0.9 : 0.7,
  }));
  const roots = ['/', '/playground'].map((path) => ({
    url: `${SITE}${path}`,
    changeFrequency: 'monthly' as const,
    priority: path === '/' ? 1 : 0.8,
  }));
  return [...roots, ...pages];
}