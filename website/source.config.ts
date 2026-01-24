import { defineConfig, defineDocs } from 'fumadocs-mdx/config';
import { metaSchema, pageSchema } from 'fumadocs-core/source/schema';
import { rehypeCodeDefaultOptions } from 'fumadocs-core/mdx-plugins';
import { codeBlockIcons } from './src/lib/language-icons';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';
import type { ShikiTransformer } from 'shiki';

const transformerLanguage: ShikiTransformer = {
  name: 'sentil:language',
  pre(node) {
    node.properties['data-language'] = this.options.lang ?? 'text';
  },
};

const COMMENT_INK = '#6a737d';

const RUNS_ON_PASTE = new Set(['bash', 'sh', 'shell', 'zsh', 'console', 'powershell', 'ps1', 'dockerfile']);

const transformerCopyNote: ShikiTransformer = {
  name: 'sentil:copy-note',
  tokens(lines) {
    if (RUNS_ON_PASTE.has(this.options.lang ?? '')) return;
    for (const line of lines) {
      let i = line.length;
      while (i > 0 && line[i - 1].htmlStyle?.['--shiki-light']?.toLowerCase() === COMMENT_INK) i -= 1;
      if (i === 0 || i === line.length) continue;
      if (line.slice(0, i).every((token) => token.content.trim() === '')) continue;
      for (const token of line.slice(i)) token.htmlAttrs = { ...token.htmlAttrs, class: 'copy-note' };
    }
  },
};

export const docs = defineDocs({
  dir: 'content/docs',
  docs: {
    schema: pageSchema,
    postprocess: {
      includeProcessedMarkdown: true,
    },
  },
  meta: {
    schema: metaSchema,
  },
});

export default defineConfig({
  mdxOptions: {
    remarkPlugins: [remarkMath],
    // Katex has to convert math nodes before the shiki pass, or a display block
    // falls through to an unknown `math` code language.
    rehypePlugins: (plugins) => [rehypeKatex, ...plugins],
    remarkStructureOptions: { mdxTypes: () => false },
    rehypeCodeOptions: {
      themes: { light: 'github-light', dark: 'dracula' },
      // rehypeCode appends its own icon transformer after this list, so the
      // marks it lacks have to come in through its option rather than through
      // a second transformer, which it would overwrite.
      icon: { extend: codeBlockIcons },
      transformers: [
        ...(rehypeCodeDefaultOptions.transformers ?? []),
        transformerLanguage,
        transformerCopyNote,
      ],
    },
  },
});