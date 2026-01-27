import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { baseOptions } from '@/lib/layout.shared';
import { SkipLink } from '@/components/skip-link';

export default function Layout({ children }: LayoutProps<'/'>) {
  const base = baseOptions();
  return (
    <>
      <SkipLink href="#main-content" />
      <HomeLayout
        {...base}
        links={[{ text: 'Get started', url: '/docs/start' }, ...(base.links ?? [])]}
      >
        {children}
      </HomeLayout>
    </>
  );
}