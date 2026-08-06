import type { AudioBeatMarker, EditorProject, MediaAsset, TimelineClip, TimelineTrack } from "../types/editor";

const EPSILON = 0.000_001;

export interface BeatMarkerAlignmentPatch {
  trackId: string;
  clipId: string;
  start: number;
  duration: number;
  trimStart: number;
  trimEnd: number;
  beatMarkers?: AudioBeatMarker[];
}

export interface BeatMarkerAlignmentPlan {
  markerTimes: number[];
  patches: BeatMarkerAlignmentPatch[];
}

interface BoundaryOption {
  time: number;
  aligned: boolean;
}

export function collectTimelineBeatMarkerTimes(project: EditorProject): number[] {
  const times = project.tracks.flatMap((track) =>
    track.clips.flatMap((clip) => {
      if (clip.type !== "audio" || clip.beatMode !== "auto" || !clip.beatMarkers?.length) {
        return [];
      }

      return clip.beatMarkers
        .filter((marker) => Number.isFinite(marker.time) && marker.time >= 0 && marker.time <= clip.duration)
        .map((marker) => clip.start + marker.time)
        .filter((time) => Number.isFinite(time) && time >= 0);
    }),
  );

  return [...new Set(times.map((time) => Number(time.toFixed(6))))].sort((left, right) => left - right);
}

export function planVideoBeatMarkerAlignment(
  project: EditorProject,
  selectedClipId: string,
  minimumDuration = 0.5,
): BeatMarkerAlignmentPlan | undefined {
  const track = project.tracks.find((item) => item.clips.some((clip) => clip.id === selectedClipId));
  const selected = track?.clips.find((clip) => clip.id === selectedClipId);
  const selectedAsset = selected ? findAsset(project, selected) : undefined;
  const originalMarkerTimes = collectTimelineBeatMarkerTimes(project);

  if (!track || track.locked || !selected || selectedAsset?.type !== "video" || originalMarkerTimes.length === 0) {
    return undefined;
  }

  const originalStart = selected.start;
  const originalEnd = selected.start + selected.duration;
  const frameTolerance = 1 / Math.max(project.fps, 1);
  const sortedClips = [...track.clips].sort((left, right) => left.start - right.start || left.id.localeCompare(right.id));
  const selectedIndex = sortedClips.findIndex((clip) => clip.id === selected.id);
  const previous = sortedClips[selectedIndex - 1];
  const next = sortedClips[selectedIndex + 1];
  const sharedPrevious = previous && Math.abs(previous.start + previous.duration - originalStart) <= frameTolerance
    ? previous
    : undefined;
  const sharedNext = next && Math.abs(originalEnd - next.start) <= frameTolerance ? next : undefined;
  const shouldPinProjectStart = track.type === "video" && selectedIndex === 0 && !sharedPrevious;
  const leadingAudioPatch = shouldPinProjectStart ? createLeadingAudioTrimPatch(project, frameTolerance) : undefined;
  const markerTimes = leadingAudioPatch
    ? collectTimelineBeatMarkerTimesWithPatch(project, leadingAudioPatch)
    : originalMarkerTimes;
  const startOptions = shouldPinProjectStart
    ? [{ time: 0, aligned: true }]
    : boundaryOptions(markerTimes, originalStart);
  const endOptions = boundaryOptions(markerTimes, originalEnd);
  let best:
    | { start: BoundaryOption; end: BoundaryOption; patches: BeatMarkerAlignmentPatch[]; alignedCount: number; distance: number }
    | undefined;

  for (const start of startOptions) {
    for (const end of endOptions) {
      const videoPatches = buildPatches(project, track, selected, sharedPrevious, sharedNext, start.time, end.time, minimumDuration);

      if (!videoPatches) {
        continue;
      }

      const patches = leadingAudioPatch ? [...videoPatches, leadingAudioPatch] : videoPatches;

      const alignedCount = Number(start.aligned) + Number(end.aligned);
      const distance = Math.abs(start.time - originalStart) + Math.abs(end.time - originalEnd);
      const isBetter = !best
        || alignedCount > best.alignedCount
        || (alignedCount === best.alignedCount && distance < best.distance - EPSILON)
        || (alignedCount === best.alignedCount
          && Math.abs(distance - best.distance) <= EPSILON
          && (start.time < best.start.time - EPSILON
            || (Math.abs(start.time - best.start.time) <= EPSILON && end.time < best.end.time - EPSILON)));

      if (isBetter) {
        best = { start, end, patches, alignedCount, distance };
      }
    }
  }

  if (!best || best.alignedCount === 0 || best.patches.length === 0) {
    return undefined;
  }

  return { markerTimes, patches: best.patches };
}

function createLeadingAudioTrimPatch(
  project: EditorProject,
  frameTolerance: number,
): BeatMarkerAlignmentPatch | undefined {
  const candidate = project.tracks
    .flatMap((track) => track.clips)
    .filter((clip) =>
      clip.type === "audio"
      && clip.beatMode === "auto"
      && Math.abs(clip.start) <= frameTolerance
      && !clip.beatMarkers?.some((marker) => Math.abs(marker.time) <= EPSILON)
      && clip.beatMarkers?.some((marker) => marker.time > EPSILON && marker.time < clip.duration),
    )
    .map((clip) => ({
      clip,
      firstMarker: Math.min(...(clip.beatMarkers ?? [])
        .filter((marker) => Number.isFinite(marker.time) && marker.time > EPSILON && marker.time < clip.duration)
        .map((marker) => marker.time)),
    }))
    .sort((left, right) => left.firstMarker - right.firstMarker || left.clip.id.localeCompare(right.clip.id))[0];

  if (!candidate || !Number.isFinite(candidate.firstMarker)) {
    return undefined;
  }

  const { clip, firstMarker } = candidate;
  const asset = findAsset(project, clip);
  const speed = Math.max(clip.speed, 0.01);
  const trimStart = clip.trimStart + firstMarker * speed;
  const duration = clip.duration - firstMarker;

  if (!asset || duration <= EPSILON || trimStart + clip.trimEnd > asset.duration + EPSILON) {
    return undefined;
  }

  return {
    trackId: clip.trackId,
    clipId: clip.id,
    start: 0,
    duration: normalizeTime(duration),
    trimStart: normalizeTime(trimStart),
    trimEnd: clip.trimEnd,
    beatMarkers: (clip.beatMarkers ?? [])
      .filter((marker) => marker.time >= firstMarker - EPSILON && marker.time <= clip.duration)
      .map((marker) => ({
        ...marker,
        time: normalizeTime(Math.max(0, marker.time - firstMarker)),
      })),
  };
}

function collectTimelineBeatMarkerTimesWithPatch(
  project: EditorProject,
  patch: BeatMarkerAlignmentPatch,
): number[] {
  const times = project.tracks.flatMap((track) =>
    track.clips.flatMap((clip) => {
      const isPatched = clip.id === patch.clipId && clip.trackId === patch.trackId;
      const start = isPatched ? patch.start : clip.start;
      const duration = isPatched ? patch.duration : clip.duration;
      const markers = isPatched ? patch.beatMarkers : clip.beatMarkers;

      if (clip.type !== "audio" || clip.beatMode !== "auto" || !markers?.length) {
        return [];
      }

      return markers
        .filter((marker) => Number.isFinite(marker.time) && marker.time >= 0 && marker.time <= duration)
        .map((marker) => start + marker.time);
    }),
  );

  return [...new Set(times.map((time) => Number(time.toFixed(6))))].sort((left, right) => left - right);
}

function boundaryOptions(markerTimes: number[], originalTime: number): BoundaryOption[] {
  return [
    ...markerTimes.map((time) => ({ time, aligned: true })),
    { time: originalTime, aligned: false },
  ];
}

function buildPatches(
  project: EditorProject,
  track: TimelineTrack,
  selected: TimelineClip,
  previous: TimelineClip | undefined,
  next: TimelineClip | undefined,
  start: number,
  end: number,
  minimumDuration: number,
): BeatMarkerAlignmentPatch[] | undefined {
  if (start < 0 || end - start < minimumDuration - EPSILON) {
    return undefined;
  }

  const selectedPatch = patchClipBoundaries(project, selected, start, end, minimumDuration);

  if (!selectedPatch) {
    return undefined;
  }

  const patches = [selectedPatch];

  if (previous) {
    const previousPatch = patchClipBoundaries(
      project,
      previous,
      previous.start,
      start,
      minimumDuration,
    );

    if (!previousPatch) {
      return undefined;
    }

    patches.push(previousPatch);
  }

  if (next) {
    const nextPatch = patchClipBoundaries(
      project,
      next,
      end,
      next.start + next.duration,
      minimumDuration,
    );

    if (!nextPatch) {
      return undefined;
    }

    patches.push(nextPatch);
  }

  return patches
    .filter((patch) => {
      const clip = track.clips.find((item) => item.id === patch.clipId);
      return clip && hasTimingChange(clip, patch);
    })
    .sort((left, right) => left.start - right.start || left.clipId.localeCompare(right.clipId));
}

function patchClipBoundaries(
  project: EditorProject,
  clip: TimelineClip,
  nextStart: number,
  nextEnd: number,
  minimumDuration: number,
): BeatMarkerAlignmentPatch | undefined {
  const asset = findAsset(project, clip);
  const speed = Math.max(clip.speed, 0.01);
  const originalEnd = clip.start + clip.duration;
  const trimStart = clip.trimStart + (nextStart - clip.start) * speed;
  const trimEnd = clip.trimEnd + (originalEnd - nextEnd) * speed;
  const duration = nextEnd - nextStart;

  if (!asset
    || nextStart < 0
    || duration < minimumDuration - EPSILON
    || trimStart < -EPSILON
    || trimEnd < -EPSILON
    || trimStart + trimEnd > asset.duration + EPSILON) {
    return undefined;
  }

  return {
    trackId: clip.trackId,
    clipId: clip.id,
    start: normalizeTime(nextStart),
    duration: normalizeTime(duration),
    trimStart: normalizeTime(Math.max(0, trimStart)),
    trimEnd: normalizeTime(Math.max(0, trimEnd)),
  };
}

function findAsset(project: EditorProject, clip: TimelineClip): MediaAsset | undefined {
  return project.assets.find((asset) => asset.id === clip.assetId);
}

function hasTimingChange(clip: TimelineClip, patch: BeatMarkerAlignmentPatch) {
  return Math.abs(clip.start - patch.start) > EPSILON
    || Math.abs(clip.duration - patch.duration) > EPSILON
    || Math.abs(clip.trimStart - patch.trimStart) > EPSILON
    || Math.abs(clip.trimEnd - patch.trimEnd) > EPSILON;
}

function normalizeTime(value: number) {
  return Number(value.toFixed(6));
}
