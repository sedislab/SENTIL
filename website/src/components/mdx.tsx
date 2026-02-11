import defaultMdxComponents from 'fumadocs-ui/mdx';
import type { MDXComponents } from 'mdx/types';
import { Tab, Tabs } from 'fumadocs-ui/components/tabs';
import { Step, Steps } from 'fumadocs-ui/components/steps';
import { Card, Cards } from '@/components/card';
import { PlaygroundLazy } from '@/components/playground/PlaygroundLazy';
import { ExerciseLazy } from '@/components/exercise/ExerciseLazy';
import { PipelineDiagram } from '@/components/PipelineDiagram';
import { OsTab, OsTabs } from '@/components/os-tabs';
import { CodeBlockWithNotes } from '@/components/code-block';

export function getMDXComponents(components?: MDXComponents) {
  return {
    ...defaultMdxComponents,
    pre: CodeBlockWithNotes,
    Tab,
    Tabs,
    Step,
    Steps,
    Card,
    Cards,
    Playground: PlaygroundLazy,
    Exercise: ExerciseLazy,
    PipelineDiagram,
    OsTabs,
    OsTab,
    ...components,
  } satisfies MDXComponents;
}

export const useMDXComponents = getMDXComponents;

declare global {
  type MDXProvidedComponents = ReturnType<typeof getMDXComponents>;
}