'use client';
import { useEffect, useRef } from 'react';
import { usePathname } from 'next/navigation';

export function ScrollTop() {
  const pathname = usePathname();
  const fromPop = useRef(false);
  const mounted = useRef(false);

  useEffect(() => {
    const onPop = () => {
      fromPop.current = true;
    };
    window.addEventListener('popstate', onPop);
    return () => window.removeEventListener('popstate', onPop);
  }, []);

  useEffect(() => {
    if (!mounted.current) {
      mounted.current = true;
      return;
    }
    if (fromPop.current) {
      fromPop.current = false;
      return;
    }
    if (window.location.hash) return;
    window.scrollTo(0, 0);
  }, [pathname]);

  return null;
}