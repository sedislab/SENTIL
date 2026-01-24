import Link from 'next/link';
import { ArrowUpRight, ChevronRight } from 'lucide-react';
import type { ReactNode } from 'react';
import { cn } from '@/lib/cn';

export function Cards({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn('not-prose my-6 grid grid-cols-1 gap-4 sm:grid-cols-2', className)}>
      {children}
    </div>
  );
}

export function Card({
  icon,
  title,
  description,
  href,
  external,
  children,
}: {
  icon?: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  href?: string;
  external?: boolean;
  children?: ReactNode;
}) {
  const inner = (
    <>
      {href && (
        <ArrowUpRight className="absolute right-5 top-5 size-4 text-fd-muted-foreground opacity-0 group-hover:text-fd-primary group-hover:opacity-100" />
      )}
      {icon && (
        <div className="mb-4 text-fd-primary [&_svg]:size-6 [&_svg]:stroke-[2.25]">{icon}</div>
      )}
      <h2 className="text-base font-semibold text-fd-foreground">{title}</h2>
      {description && (
        <p className="mt-1 text-base leading-6 text-fd-muted-foreground">{description}</p>
      )}
      {children && (
        <div className="mt-4 flex flex-nowrap items-center gap-1 whitespace-nowrap text-sm font-medium text-fd-muted-foreground transition-colors group-hover:text-fd-primary [&>p]:m-0 [&>p]:contents">
          {children}
          <ChevronRight className="size-3.5 shrink-0" />
        </div>
      )}
    </>
  );

  const className =
    'group relative flex flex-col rounded-2xl border border-fd-border bg-fd-card px-6 py-5 ring-2 ring-transparent no-underline hover:no-underline transition-colors hover:border-fd-primary';

  if (!href) return <div className={className}>{inner}</div>;

  if (external) {
    return (
      <a href={href} data-card target="_blank" rel="noreferrer noopener" className={className}>
        {inner}
      </a>
    );
  }

  return (
    <Link href={href} data-card prefetch={false} className={className}>
      {inner}
    </Link>
  );
}