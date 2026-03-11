import { getPageImage, getPageMarkdownUrl, source } from '@/lib/source';
import {
  DocsBody,
  DocsDescription,
  DocsPage,
  DocsTitle,
} from 'fumadocs-ui/layouts/notebook/page';
import { notFound } from 'next/navigation';
import { Pencil } from 'lucide-react';
import { getMDXComponents } from '@/components/mdx';
import type { Metadata } from 'next';
import { createRelativeLink } from 'fumadocs-ui/mdx';
import { CopyPage } from '@/components/copy-page';
import type { ReactNode } from 'react';

const SITE = 'https://sentil.pages.dev';

function parentSection(nodes: any[], url: string, name: ReactNode): ReactNode {
  for (const node of nodes) {
    if (node.type === 'page' && node.url === url) return name;
    if (node.type === 'folder') {
      if (node.index?.url === url) return name;
      const found = parentSection(node.children ?? [], url, node.name);
      if (found != null) return found;
    }
  }
  return null;
}

export default async function Page(props: PageProps<'/docs/[[...slug]]'>) {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  const MDX = page.data.body;
  const markdownUrl = getPageMarkdownUrl(page).url;
  const editUrl = `https://github.com/sedislab/SENTIL/edit/main/website/content/docs/${page.path}`;
  const eyebrow = parentSection(source.getPageTree().children, page.url, null);

  const jsonLd = [
    {
      '@context': 'https://schema.org',
      '@type': 'TechArticle',
      headline: page.data.title,
      description: page.data.description,
      url: `${SITE}${page.url}`,
      inLanguage: 'en',
      isPartOf: { '@id': `${SITE}/#website` },
      author: { '@id': `${SITE}/#org` },
      publisher: { '@id': `${SITE}/#org` },
    },
  ];

  return (
    <DocsPage toc={page.data.toc} full={page.data.full} breadcrumb={{ enabled: false }} role="main" tabIndex={-1}>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      {eyebrow && (
        <p className="mb-1.5 text-sm font-semibold leading-4 tracking-[0.01em] text-fd-primary">
          {eyebrow}
        </p>
      )}
      <div className="@container/page-header flex flex-row items-center justify-between gap-4">
        <DocsTitle className="mb-0">{page.data.title}</DocsTitle>
        <div className="hidden @[520px]/page-header:flex">
          <CopyPage markdownUrl={markdownUrl} editUrl={editUrl} />
        </div>
      </div>
      <DocsDescription className="page-subtitle">{page.data.description}</DocsDescription>
      <DocsBody>
        <MDX
          components={getMDXComponents({
            a: createRelativeLink(source, page),
          })}
        />
      </DocsBody>
      <a
        href={editUrl}
        target="_blank"
        rel="noreferrer noopener"
        className="mt-8 inline-flex w-fit items-center gap-1.5 text-sm text-fd-muted-foreground transition-colors hover:text-fd-primary"
      >
        <Pencil className="size-3.5" />
        Edit this page on GitHub
      </a>
    </DocsPage>
  );
}

export async function generateStaticParams() {
  return source.generateParams();
}

export async function generateMetadata(props: PageProps<'/docs/[[...slug]]'>): Promise<Metadata> {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  return {
    title: page.data.title,
    description: page.data.description,
    alternates: { canonical: page.url },
    openGraph: {
      images: [{ url: getPageImage(page).url, alt: page.data.title }],
    },
  };
}