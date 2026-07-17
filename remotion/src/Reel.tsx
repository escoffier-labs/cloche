import React from 'react';
import {
  AbsoluteFill,
  Easing,
  OffthreadVideo,
  interpolate,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from 'remotion';
import {ShaderGrainGradient} from './remocn/shader-grain-gradient';
import {StaggeredWords} from './remocn/staggered-words';

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
  /** Optional focus point, 0..1 across the footage. Defaults to center. */
  x?: number;
  y?: number;
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
  fps: 60,
  width: 1080,
  height: 1920,
  cues: {},
};

// Ease-out for entrances: arrive fast, settle gently.
const EASE_ENTER = Easing.bezier(0.16, 1, 0.3, 1);
// Ease-in for exits: leave with gravity.
const EASE_EXIT = Easing.in(Easing.cubic);
// Balanced in-out for camera moves so zooms breathe instead of lurching.
const EASE_ZOOM = Easing.bezier(0.45, 0, 0.55, 1);

/**
 * 0..1 envelope over a [startMs, endMs] window: eased ramp in, hold, eased
 * ramp out that lands exactly at endMs. Pure function of `nowMs`, so parallel
 * render tabs always agree on a frame.
 */
const cueEnvelope = (
  nowMs: number,
  startMs: number,
  endMs: number,
  rampInMs: number,
  rampOutMs: number,
  easeIn: (t: number) => number,
  easeOut: (t: number) => number,
): number => {
  const len = Math.max(1, endMs - startMs);
  const rampIn = Math.min(rampInMs, len / 2);
  const rampOut = Math.min(rampOutMs, len / 2);
  const enter = interpolate(nowMs, [startMs, startMs + rampIn], [0, 1], {
    easing: easeIn,
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const exit = interpolate(nowMs, [endMs - rampOut, endMs], [0, 1], {
    easing: easeOut,
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  return enter * (1 - exit);
};

type ZoomState = {
  scale: number;
  originX: number;
  originY: number;
};

/** Blend all zoom cues into one smooth camera state for this instant. */
const zoomStateAt = (nowMs: number, zooms: ZoomCue[] | undefined): ZoomState => {
  let scale = 1;
  let originX = 0.5;
  let originY = 0.5;
  let strongest = 0;
  for (const cue of zooms ?? []) {
    const target = cue.scale ?? 1;
    if (target <= 1) {
      continue;
    }
    const envelope = cueEnvelope(
      nowMs,
      cue.startMs,
      cue.endMs,
      450,
      450,
      EASE_ZOOM,
      EASE_ZOOM,
    );
    const contribution = (target - 1) * envelope;
    if (contribution > scale - 1) {
      scale = 1 + contribution;
    }
    if (envelope > strongest) {
      strongest = envelope;
      originX = cue.x ?? 0.5;
      originY = cue.y ?? 0.5;
    }
  }
  return {scale, originX, originY};
};

export const ClocheReel: React.FC<ReelProps> = ({inputVideo, title, cues}) => {
  const frame = useCurrentFrame();
  const {fps, durationInFrames} = useVideoConfig();
  const nowMs = (frame / fps) * 1000;
  const totalMs = (durationInFrames / fps) * 1000;
  const titleMs = cues.titleCard?.ms ?? 900;
  const outroMs = cues.outroCard?.ms ?? 0;

  // Title and outro are scenes, not overlays: the title covers fully at frame
  // 0 and releases into the footage; the outro takes over and HOLDS to the
  // final frame. The footage stage fades and sinks while covered, so nothing
  // ever ghosts over running footage.
  const titleCover =
    titleMs > 0
      ? 1 -
        interpolate(nowMs, [Math.max(0, titleMs - 400), titleMs], [0, 1], {
          easing: EASE_ZOOM,
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })
      : 0;
  const outroStart = totalMs - outroMs;
  const outroCover =
    outroMs > 0
      ? interpolate(nowMs, [outroStart, Math.min(totalMs, outroStart + 400)], [0, 1], {
          easing: EASE_ZOOM,
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })
      : 0;
  const cover = Math.max(titleCover, outroCover);
  // Fade through the brand background instead of crossfading: the footage is
  // gone by cover 0.55, the card text only appears after cover 0.45, so text
  // never sits over bright footage.
  const stageVis = 1 - Math.min(1, cover * 1.8);
  const textVis = Math.max(0, (cover - 0.45) / 0.55);

  const zoom = zoomStateAt(nowMs, cues.zooms);
  // Slow drift on the whole stage so held shots never feel frozen.
  const drift = interpolate(nowMs, [0, totalMs], [1, 1.015], {
    easing: EASE_ZOOM,
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const stageScale = drift * (1 - 0.06 * cover);
  const videoSource = inputVideo.startsWith('http') ? inputVideo : staticFile(inputVideo);

  return (
    <AbsoluteFill style={styles.canvas}>
      <ShaderGrainGradient
        colors={['#235f62', '#3d7f8a', '#f28b54']}
        colorBack="#0d1a28"
        softness={0.72}
        intensity={0.16}
        noise={0.06}
        speed={0.55}
      />
      <div
        style={{
          ...styles.stage,
          opacity: stageVis,
          transform: `scale(${stageScale})`,
        }}
      >
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
                  transform: `scale(${zoom.scale})`,
                  transformOrigin: `${zoom.originX * 100}% ${zoom.originY * 100}%`,
                }}
              />
            ) : (
              <div style={styles.empty}>Missing input video</div>
            )}
          </div>
        </div>
      </div>
      {cues.captions?.map((cue, index) => (
        <Caption key={index} cue={cue} nowMs={nowMs} />
      ))}
      {titleCover >= outroCover && textVis > 0.004 ? (
        <CardScene
          text={cues.titleCard?.text ?? title}
          cover={textVis}
          revealMs={nowMs}
          exitLift={-26}
        />
      ) : null}
      {outroCover > titleCover && textVis > 0.004 ? (
        <CardScene
          text={cues.outroCard?.text ?? title}
          cover={textVis}
          revealMs={nowMs - (outroStart + 180)}
          exitLift={26}
        />
      ) : null}
    </AbsoluteFill>
  );
};

const Caption: React.FC<{cue: CaptionCue; nowMs: number}> = ({cue, nowMs}) => {
  const envelope = cueEnvelope(
    nowMs,
    cue.startMs,
    cue.endMs,
    240,
    200,
    EASE_ENTER,
    EASE_EXIT,
  );
  if (envelope <= 0.004) {
    return null;
  }
  const top = cue.position === 'top';
  // Bottom captions rise into place; top captions drop in.
  const offset = (1 - envelope) * (top ? -18 : 18);
  return (
    <div
      style={{
        ...styles.caption,
        ...(top ? styles.captionTop : styles.captionBottom),
        opacity: envelope,
        transform: `translateY(${offset}px)`,
      }}
    >
      {cue.text}
    </div>
  );
};

/**
 * Full-screen title/outro scene. The text sits directly on the brand canvas
 * (the footage stage fades out underneath), so partial opacity during the
 * transition reads as a scene change instead of ghost text over footage.
 * Words rise in staggered from `revealMs`; the whole block lifts away with
 * `exitLift` as the cover releases.
 */
const CardScene: React.FC<{
  text: string;
  cover: number;
  revealMs: number;
  exitLift: number;
}> = ({text, cover, revealMs, exitLift}) => {
  const lift = exitLift * (1 - cover);
  return (
    <AbsoluteFill style={{...styles.card, opacity: cover}}>
      <div style={{...styles.cardText, transform: `translateY(${lift}px)`}}>
        <StaggeredWords text={text} localMs={revealMs} />
      </div>
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
    display: 'flex',
    justifyContent: 'center',
    padding: 96,
  },
  cardText: {
    maxWidth: 860,
    color: '#ffffff',
    fontSize: 84,
    fontWeight: 900,
    lineHeight: 1.04,
    letterSpacing: '-0.01em',
    textAlign: 'center',
    textShadow: '0 10px 44px rgba(4, 10, 16, 0.6), 0 2px 10px rgba(4, 10, 16, 0.4)',
  },
};
