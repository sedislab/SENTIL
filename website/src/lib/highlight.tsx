import { codeToHast } from 'shiki';
import { toJsxRuntime } from 'hast-util-to-jsx-runtime';
import { Fragment, jsx, jsxs } from 'react/jsx-runtime';

export async function Highlight({ code, lang }: { code: string; lang: string }) {
  const hast = await codeToHast(code, {
    lang,
    themes: { light: 'github-light', dark: 'dracula' },
    defaultColor: false,
  });
  return toJsxRuntime(hast, { Fragment, jsx, jsxs });
}