import React from 'react';
import {
  AbsoluteFill,
  OffthreadVideo,
  interpolate,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from 'remotion';

type CaptionCue = {
  startMs: number;
  endMs: number;
  text: string;
  position?: 'top' | 'bottom';
};

type ZoomCue = {
  startMs: number;
  endMs: number;
  scale?: number;
};

type CardCue = {
  text?: string;
  ms?: number;
};

type ReelCues = {
  captions?: CaptionCue[];
  zooms?: ZoomCue[];
  titleCard?: CardCue;
  outroCard?: CardCue;
};

export type ReelProps = {
  inputVideo: string;
  title: string;
  durationMs: number;
  fps: number;
  width: number;
  height: number;
  cues: ReelCues;
};

export const defaultReelProps: ReelProps = {
  inputVideo: '',
  title: 'Cloche Reel',
  durationMs: 6000,
  fps: 30,
  width: 1080,
  height: 1920,
  cues: {},
};

export const ClocheReel: React.FC<ReelProps> = ({inputVideo, title, cues}) => {
  const frame = useCurrentFrame();
  const {fps, durationInFrames} = useVideoConfig();
  const nowMs = (frame / fps) * 1000;
  const activeCaption = cues.captions?.find(
    (caption) => nowMs >= caption.startMs && nowMs < caption.endMs,
  );
  const activeZoom = cues.zooms?.find((zoom) => nowMs >= zoom.startMs && nowMs < zoom.endMs);
  const titleMs = cues.titleCard?.ms ?? 900;
  const outroMs = cues.outroCard?.ms ?? 0;
  const remainingMs = ((durationInFrames - frame) / fps) * 1000;
  const videoScale = activeZoom?.scale
    ? interpolate(
        nowMs,
        [activeZoom.startMs, activeZoom.startMs + 300, activeZoom.endMs],
        [1, activeZoom.scale, activeZoom.scale],
        {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'},
      )
    : 1;
  const videoSource = inputVideo.startsWith('http') ? inputVideo : staticFile(inputVideo);

  return (
    <AbsoluteFill style={styles.canvas}>
      <div style={styles.backdropA} />
      <div style={styles.backdropB} />
      <div style={styles.stage}>
        <div style={styles.chrome}>
          <div style={styles.toolbar}>
            <span style={{...styles.dot, backgroundColor: '#ff6b6b'}} />
            <span style={{...styles.dot, backgroundColor: '#feca57'}} />
            <span style={{...styles.dot, backgroundColor: '#1dd1a1'}} />
            <span style={styles.toolbarTitle}>{title}</span>
          </div>
          <div style={styles.videoShell}>
            {inputVideo ? (
              <OffthreadVideo
                src={videoSource}
                muted
                style={{
                  ...styles.video,
                  transform: `scale(${videoScale})`,
                }}
              />
            ) : (
              <div style={styles.empty}>Missing input video</div>
            )}
          </div>
        </div>
      </div>
      {activeCaption ? <Caption cue={activeCaption} /> : null}
      {nowMs < titleMs ? (
        <Card text={cues.titleCard?.text ?? title} progress={1 - nowMs / titleMs} />
      ) : null}
      {outroMs > 0 && remainingMs < outroMs ? (
        <Card text={cues.outroCard?.text ?? title} progress={1 - remainingMs / outroMs} />
      ) : null}
    </AbsoluteFill>
  );
};

const Caption: React.FC<{cue: CaptionCue}> = ({cue}) => {
  const top = cue.position === 'top';
  return (
    <div style={{...styles.caption, ...(top ? styles.captionTop : styles.captionBottom)}}>
      {cue.text}
    </div>
  );
};

const Card: React.FC<{text: string; progress: number}> = ({text, progress}) => {
  const opacity = interpolate(progress, [0, 0.18, 0.82, 1], [0, 1, 1, 0], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  return (
    <AbsoluteFill style={{...styles.card, opacity}}>
      <div style={styles.cardText}>{text}</div>
    </AbsoluteFill>
  );
};

const styles: Record<string, React.CSSProperties> = {
  canvas: {
    background: 'linear-gradient(140deg, #102033 0%, #235f62 52%, #f28b54 100%)',
    color: '#f8fbff',
    fontFamily:
      'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    overflow: 'hidden',
  },
  backdropA: {
    position: 'absolute',
    inset: '4% 14% 56% 8%',
    background: 'linear-gradient(135deg, rgba(249, 202, 98, 0.5), rgba(64, 176, 166, 0.18))',
    filter: 'blur(52px)',
    transform: 'rotate(-12deg)',
  },
  backdropB: {
    position: 'absolute',
    inset: '48% 8% 7% 18%',
    background: 'linear-gradient(135deg, rgba(69, 113, 178, 0.45), rgba(255, 255, 255, 0.16))',
    filter: 'blur(58px)',
    transform: 'rotate(9deg)',
  },
  stage: {
    position: 'absolute',
    inset: 88,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  chrome: {
    width: '100%',
    maxHeight: '100%',
    aspectRatio: '16 / 10',
    borderRadius: 24,
    overflow: 'hidden',
    background: '#101820',
    boxShadow: '0 44px 110px rgba(4, 12, 18, 0.46), 0 18px 38px rgba(4, 12, 18, 0.34)',
  },
  toolbar: {
    height: 54,
    display: 'flex',
    alignItems: 'center',
    gap: 12,
    padding: '0 20px',
    background: 'rgba(11, 18, 24, 0.94)',
  },
  dot: {
    width: 14,
    height: 14,
    borderRadius: 99,
    flex: '0 0 auto',
  },
  toolbarTitle: {
    marginLeft: 12,
    color: 'rgba(248, 251, 255, 0.78)',
    fontSize: 18,
    fontWeight: 700,
    lineHeight: 1,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  videoShell: {
    position: 'relative',
    height: 'calc(100% - 54px)',
    background: '#070b10',
    overflow: 'hidden',
  },
  video: {
    width: '100%',
    height: '100%',
    objectFit: 'contain',
    transformOrigin: 'center',
  },
  empty: {
    height: '100%',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    color: 'rgba(248, 251, 255, 0.78)',
    fontSize: 32,
    fontWeight: 800,
  },
  caption: {
    position: 'absolute',
    left: 96,
    right: 96,
    padding: '24px 30px',
    borderRadius: 20,
    background: 'rgba(7, 13, 19, 0.78)',
    border: '1px solid rgba(255, 255, 255, 0.18)',
    boxShadow: '0 20px 42px rgba(0, 0, 0, 0.28)',
    fontSize: 42,
    fontWeight: 850,
    lineHeight: 1.14,
    textAlign: 'center',
  },
  captionTop: {
    top: 116,
  },
  captionBottom: {
    bottom: 116,
  },
  card: {
    alignItems: 'center',
    background: 'rgba(6, 11, 16, 0.84)',
    display: 'flex',
    justifyContent: 'center',
    padding: 96,
  },
  cardText: {
    maxWidth: 820,
    color: '#ffffff',
    fontSize: 74,
    fontWeight: 900,
    lineHeight: 1.02,
    textAlign: 'center',
  },
};
