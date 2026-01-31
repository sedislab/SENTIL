'use client';

import dynamic from 'next/dynamic';
import type { ComponentProps } from 'react';
import type Exercise from './Exercise';

const ExerciseImpl = dynamic(() => import('./Exercise'), {
  loading: () => (
    <div className="exercise not-prose">
      <p className="exercise-label">Exercise</p>
    </div>
  ),
});

export function ExerciseLazy(props: ComponentProps<typeof Exercise>) {
  return <ExerciseImpl {...props} />;
}