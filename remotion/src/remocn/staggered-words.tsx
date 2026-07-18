// Adapted from remocn's staggered-fade-up (https://github.com/Remocn/remocn),
// MIT License. Words rise into place one after another. Unlike the original
// this renders transparent and is driven by an explicit local time in ms, so
// the caller (a scene cover) controls when the reveal starts.
import React from 'react';
import {Easing, interpolate} from 'remotion';

const EASE_RISE = Easing.bezier(0.16, 1, 0.3, 1);

export const StaggeredWords: React.FC<{
  text: string;
  /** Milliseconds since the reveal started; words before 0 are hidden. */
  localMs: number;
  staggerMs?: number;
  riseMs?: number;
  distance?: number;
  style?: React.CSSProperties;
}> = ({text, localMs, staggerMs = 70, riseMs = 380, distance = 26, style}) => {
  const words = text.split(' ');
  return (
    <span style={style}>
      {words.map((word, i) => {
        const local = localMs - i * staggerMs;
        const progress = interpolate(local, [0, riseMs], [0, 1], {
          easing: EASE_RISE,
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        });
        return (
          <span
            key={i}
            style={{
              display: 'inline-block',
              marginRight: '0.28em',
              opacity: progress,
              transform: `translateY(${(1 - progress) * distance}px)`,
            }}
          >
            {word}
          </span>
        );
      })}
    </span>
  );
};
