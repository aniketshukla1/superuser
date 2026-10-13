// Encodes the demo video's frames into an MP4 (H.264) with macOS's own encoder, so making
// the video needs nothing beyond the Xcode command line tools the app already needs.
//
//   swift scripts/encode-video.swift <frames-dir> <out.mp4> [width] [bits-per-second]
//
// <frames-dir> holds the JPEGs and manifest.json that surfaces/gui/e2e-demo/video.spec.ts
// writes, one [file, timestamp] per repaint and a final ["end", timestamp]. Each frame is
// shown from its own timestamp until the next, so a still screen costs almost nothing and
// motion keeps the timing it had in the browser.
import AVFoundation
import CoreGraphics
import Foundation
import ImageIO

let args = CommandLine.arguments
guard args.count >= 3 else {
  print("usage: swift encode-video.swift <frames-dir> <out.mp4> [width] [bits-per-second]")
  exit(2)
}
let dir = URL(fileURLWithPath: args[1])
let out = URL(fileURLWithPath: args[2])
let width = args.count > 3 ? Int(args[3])! : 1600
let bitrate = args.count > 4 ? Int(args[4])! : 2_500_000

struct Entry: Decodable {
  let file: String
  let t: Double
  init(from decoder: Decoder) throws {
    var c = try decoder.unkeyedContainer()
    file = try c.decode(String.self)
    t = try c.decode(Double.self)
  }
}
let entries = try JSONDecoder().decode([Entry].self, from: Data(contentsOf: dir.appendingPathComponent("manifest.json")))
let frames = entries.filter { $0.file != "end" }
guard let first = frames.first, let endMark = entries.last else { exit(1) }

func load(_ name: String) -> CGImage {
  let src = CGImageSourceCreateWithURL(dir.appendingPathComponent(name) as CFURL, nil)!
  return CGImageSourceCreateImageAtIndex(src, 0, nil)!
}
let probe = load(first.file)
let height = Int((Double(width) * Double(probe.height) / Double(probe.width) / 2).rounded()) * 2

try? FileManager.default.removeItem(at: out)
let writer = try AVAssetWriter(outputURL: out, fileType: .mp4)
writer.shouldOptimizeForNetworkUse = true  // index first, so a browser can start playing at once
let input = AVAssetWriterInput(mediaType: .video, outputSettings: [
  AVVideoCodecKey: AVVideoCodecType.h264,
  AVVideoWidthKey: width,
  AVVideoHeightKey: height,
  AVVideoCompressionPropertiesKey: [
    AVVideoAverageBitRateKey: bitrate,
    AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel,
    AVVideoMaxKeyFrameIntervalKey: 90,
  ],
])
input.expectsMediaDataInRealTime = false
let adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: [
  kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
  kCVPixelBufferWidthKey as String: width,
  kCVPixelBufferHeightKey as String: height,
])
writer.add(input)
writer.startWriting()
writer.startSession(atSourceTime: .zero)

func pixels(_ image: CGImage) -> CVPixelBuffer {
  var made: CVPixelBuffer?
  CVPixelBufferPoolCreatePixelBuffer(nil, adaptor.pixelBufferPool!, &made)
  let buf = made!
  CVPixelBufferLockBaseAddress(buf, [])
  let ctx = CGContext(
    data: CVPixelBufferGetBaseAddress(buf), width: width, height: height, bitsPerComponent: 8,
    bytesPerRow: CVPixelBufferGetBytesPerRow(buf), space: CGColorSpaceCreateDeviceRGB(),
    bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)!
  ctx.interpolationQuality = .high
  ctx.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
  CVPixelBufferUnlockBaseAddress(buf, [])
  return buf
}

func append(_ file: String, at seconds: Double) {
  while !input.isReadyForMoreMediaData { usleep(2000) }
  adaptor.append(pixels(load(file)), withPresentationTime: CMTime(seconds: seconds, preferredTimescale: 6000))
}

// Repaints closer together than 1/60 s are merged: the later one stands for both.
var shown = -1.0
var pending: (String, Double)? = nil
for f in frames {
  let t = f.t - first.t
  if let (file, at) = pending, t - at >= 1.0 / 60 {
    append(file, at: at)
    shown = at
  }
  pending = (f.file, shown < 0 ? 0 : max(t, shown + 1.0 / 60))
}
if let (file, at) = pending { append(file, at: at); shown = at }
let end = max(endMark.t - first.t, shown + 0.1)
append(frames.last!.file, at: end)  // hold the last screen until the recording stopped

input.markAsFinished()
let finished = DispatchSemaphore(value: 0)
writer.finishWriting { finished.signal() }
finished.wait()
guard writer.status == .completed else {
  print("encoding failed:", writer.error.map { "\($0)" } ?? "unknown")
  exit(1)
}
let bytes = (try FileManager.default.attributesOfItem(atPath: out.path)[.size] as! NSNumber).intValue
print("wrote \(out.lastPathComponent): \(width)x\(height), \(String(format: "%.1f", end)) s, \(bytes / 1024) KB")
