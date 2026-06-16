import React from 'react';
import {Composition, getInputProps} from 'remotion';
import {ClocheReel, ReelProps, defaultReelProps} from './Reel';

export const RemotionRoot: React.FC = () => {
  const inputProps = getInputProps() as Partial<ReelProps>;
  const props: ReelProps = {
    ...defaultReelProps,
    ...inputProps,
    cues: inputProps.cues ?? defaultReelProps.cues,
  };
  const fps = props.fps || defaultReelProps.fps;
  const durationInFrames = Math.max(1, Math.ceil((props.durationMs / 1000) * fps));

  return (
    <Composition
      id="ClocheReel"
      component={ClocheReel}
      durationInFrames={durationInFrames}
      fps={fps}
      width={props.width}
      height={props.height}
      defaultProps={props}
    />
  );
};
