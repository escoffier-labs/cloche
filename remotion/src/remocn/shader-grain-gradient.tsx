// Adapted from remocn (https://github.com/Remocn/remocn), MIT License.
// Deterministic animated grain-gradient background: the shader is paused
// (speed 0) and driven explicitly from the Remotion frame, so every render
// tab agrees on a frame and the animation is a pure function of time.
import React from 'react';
import {GrainGradient, type GrainGradientProps} from '@paper-design/shaders-react';
import {continueRender, delayRender, useCurrentFrame, useVideoConfig} from 'remotion';

export type ShaderGrainGradientProps = Omit<GrainGradientProps, 'frame' | 'ref'> & {
  speed?: number;
};

export const ShaderGrainGradient: React.FC<ShaderGrainGradientProps> = ({
  speed = 1,
  colors,
  colorBack = '#12121a',
  softness = 0.6,
  intensity = 0.2,
  noise = 0.15,
  ...rest
}) => {
  const frame = useCurrentFrame();
  const {fps, width, height} = useVideoConfig();

  const [handle] = React.useState(() => delayRender('shader-grain-gradient'));
  const gate = React.useCallback(
    (element: HTMLDivElement | null) => {
      if (!element) {
        return;
      }
      requestAnimationFrame(() => requestAnimationFrame(() => continueRender(handle)));
    },
    [handle],
  );

  return (
    <div ref={gate} style={{position: 'absolute', inset: 0}}>
      <GrainGradient
        speed={0}
        frame={(frame / fps) * speed * 1000}
        colors={colors}
        colorBack={colorBack}
        softness={softness}
        intensity={intensity}
        noise={noise}
        fit="cover"
        width={width}
        height={height}
        {...rest}
      />
    </div>
  );
};
