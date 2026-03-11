export const dynamic = 'force-static';

import type { MetadataRoute } from 'next';

const SITE = 'https://sentil.pages.dev';

export default function robots(): MetadataRoute.Robots {
  return {
    rules: { userAgent: '*', allow: '/' },
    sitemap: `${SITE}/sitemap.xml`,
  };
}