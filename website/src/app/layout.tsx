import { Inter, JetBrains_Mono } from 'next/font/google';
import localFont from 'next/font/local';
import type { Metadata } from 'next';
import { Provider } from '@/components/provider';
import './global.css';

const SITE = 'https://sentil.pages.dev';

export const metadata: Metadata = {
  metadataBase: new URL(SITE),
  title: {
    default: 'SENTIL: runtime verification for signal temporal logic',
    template: '%s | SENTIL',
  },
  description:
    'A toolkit for signal temporal logic and its probabilistic extension. Monitor a signal in real time, verify a property under noise, and synthesize a controller from a specification.',
  keywords: [
    'signal temporal logic',
    'STL monitoring',
    'runtime verification',
    'probabilistic signal temporal logic',
    'PrSTL',
    'statistical model checking',
    'STL robustness',
    'controller synthesis',
    'STL falsification',
  ],
  applicationName: 'SENTIL',
  authors: [{ name: 'Paapa Kwesi Quansah' }, { name: 'Ernest Bonnah' }],
  openGraph: {
    type: 'website',
    siteName: 'SENTIL',
    url: SITE,
    title: 'SENTIL: runtime verification for signal temporal logic',
    description:
      'Monitor a signal in real time, verify a property under noise, and synthesize a controller from a specification.',
  },
  twitter: { card: 'summary_large_image' },
  alternates: { canonical: '/' },
};

const jsonLd = {
  '@context': 'https://schema.org',
  '@graph': [
    {
      '@type': 'WebSite',
      '@id': `${SITE}/#website`,
      url: SITE,
      name: 'SENTIL',
      description:
        'Runtime verification and controller synthesis for signal temporal logic and its probabilistic extension.',
      publisher: { '@id': `${SITE}/#org` },
    },
    {
      '@type': 'Organization',
      '@id': `${SITE}/#org`,
      name: 'SEDIS Lab',
      url: SITE,
    },
    {
      '@type': 'SoftwareSourceCode',
      name: 'SENTIL',
      description:
        'A runtime verification tool for probabilistic signal temporal logic, with a Rust core and bindings across many languages and platforms.',
      codeRepository: 'https://github.com/sedislab/SENTIL',
      programmingLanguage: ['Rust', 'Python', 'C', 'C++', 'Java', 'Julia', 'MATLAB'],
      license: 'https://spdx.org/licenses/MIT.html',
      author: { '@id': `${SITE}/#org` },
    },
    {
      '@type': 'SoftwareApplication',
      '@id': `${SITE}/#app`,
      name: 'SENTIL',
      applicationCategory: 'DeveloperApplication',
      operatingSystem: 'Linux, macOS, Windows',
      softwareVersion: '0.3.0',
      url: SITE,
      downloadUrl: 'https://github.com/sedislab/SENTIL/releases',
      license: 'https://spdx.org/licenses/MIT.html',
      author: { '@id': `${SITE}/#org` },
    },
  ],
};

const inter = Inter({ subsets: ['latin'], variable: '--font-inter', display: 'swap' });

const mono = JetBrains_Mono({
  subsets: ['latin'],
  weight: ['400', '700'],
  variable: '--font-jbmono',
  display: 'swap',
  preload: false,
});

const display = localFont({
  src: '../fonts/inter-display-700.woff2',
  weight: '700',
  variable: '--font-display',
  display: 'swap',
});

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    <html
      lang="en"
      className={`${inter.variable} ${display.variable} ${mono.variable}`}
      suppressHydrationWarning
    >
      <body className="flex flex-col min-h-screen">
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
        <Provider>{children}</Provider>
      </body>
    </html>
  );
}