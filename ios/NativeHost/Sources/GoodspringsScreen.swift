import SwiftUI
import UniformTypeIdentifiers
import simd

/// Real ESM-derived location gets the main screen. The former gallery is
/// reachable only from Developer Preview rather than being the home screen.
struct GoodspringsScreen: View {
    let onOpenViewer: () -> Void
    @State private var importerVisible = false
    @State private var loadedWorld: GoodspringsWorld?
    @State private var joystick = SIMD2<Float>(0,0)
    @State private var loading = false
    @State private var attemptedOfflineRestore = false
    @State private var status = "Load a real FalloutNV.esm interior exported from your installed PC game."
    @State private var sceneName = "Prospector Saloon · Goodsprings"

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading,spacing:2) {
                    Text("FALLOUT: NEW VEGAS").font(.system(.headline,design:.monospaced))
                    Text(sceneName).font(.footnote).foregroundStyle(.secondary)
                }
                Spacer()
                Button("Developer Preview",action:onOpenViewer)
                    .font(.caption)
            }
            .padding(.horizontal,14)
            .padding(.vertical,10)
            if let world = loadedWorld {
                GeometryReader { _ in
                    ZStack {
                        GoodspringsWalkView(world:world,move:joystick)
                            .ignoresSafeArea(edges:.bottom)
                        // HUD and touch controls are separate from the GPU view,
                        // so dragging the right side changes the look direction.
                        VStack {
                            HStack {
                                Text("CELL: \(world.cellName)")
                                    .font(.system(.caption2,design:.monospaced))
                                    .padding(7)
                                    .background(.black.opacity(0.65),in:RoundedRectangle(cornerRadius:6))
                                Spacer()
                                VStack(alignment:.trailing,spacing:2) {
                                    Text("\(world.instances.count) REFR · FNVWLD v\(world.formatVersion)")
                                    Text("\(world.texturedBatchCount)/\(world.totalBatchCount) textured material batches")
                                }
                                    .font(.system(.caption2,design:.monospaced))
                                    .padding(7)
                                    .background(.black.opacity(0.65),in:RoundedRectangle(cornerRadius:6))
                            }
                            .foregroundStyle(.white)
                            Spacer()
                            HStack(alignment:.bottom) {
                                MoveStick(value:$joystick)
                                Spacer()
                                VStack(alignment:.trailing,spacing:5) {
                                    Text("DRAG SCREEN / RIGHT STICK TO LOOK")
                                    Text("TOUCH / GAMEPAD LEFT STICK: MOVE")
                                    Text("FIRST PERSON · NATIVE METAL")
                                }
                                .font(.system(size:9,design:.monospaced))
                                .foregroundStyle(.white.opacity(0.85))
                                .padding(8)
                                .background(.black.opacity(0.65),in:RoundedRectangle(cornerRadius:6))
                                .padding(.bottom,20)
                            }
                        }
                        .padding(12)
                        Image(systemName:"plus")
                            .font(.system(size:16,weight:.light))
                            .foregroundStyle(.white.opacity(0.70))
                            .allowsHitTesting(false)
                    }
                }
                HStack {
                    Button("Load different location") {
                        importerVisible = true
                    }
                    .font(.caption)
                    Spacer()
                    Text("World geometry preview · collision, AI, combat not integrated")
                        .font(.system(size:9))
                        .foregroundStyle(.secondary)
                }
                .padding(.horizontal,10)
                .padding(.vertical,7)
            } else {
                Spacer()
                VStack(spacing:16) {
                    Image(systemName:"building.2.crop.circle")
                        .font(.system(size:46,weight:.thin))
                    Text("ENTER GOODSPRINGS")
                        .font(.title2.bold())
                    Text("The real Prospector Saloon interior from your installed Fallout: New Vegas data. Original world placements, native rendering, first-person movement. Import once; the world remains on your iPhone.")
                        .font(.subheadline)
                        .multilineTextAlignment(.center)
                        .foregroundStyle(.secondary)
                    Button(loading ? "Loading world..." : "Install Goodsprings world (one time)") {
                        importerVisible = true
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(loading)
                    if loading {
                        ProgressView("Parsing genuine New Vegas meshes and DDS textures")
                    }
                    Text(status)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                }
                .padding(.horizontal,26)
                Spacer()
            }
        }
        .onAppear {
            // This is a standalone local app. Load the last installed game
            // scene automatically on later offline launches without a PC,
            // internet access, or another Files picker interaction.
            guard !attemptedOfflineRestore else { return }
            attemptedOfflineRestore = true
            let existing = cachedWorldURL()
            if FileManager.default.fileExists(atPath:existing.path) {
                openWorld(existing, persist: false)
            }
        }
        .fileImporter(
            isPresented:$importerVisible,
            allowedContentTypes:[.data,
                UTType(filenameExtension:"fnvworld",conformingTo:.data) ?? .data]
            // One Files importer, with an explicit custom extension just as
            // the physically verified Phase 9 .fnvpack picker requires.
        ) { selection in
            switch selection {
            case .failure(let error):
                status = "Files import failed: \(error.localizedDescription)"
            case .success(let url):
                openWorld(url)
            }
        }
    }

    /// Private original-data cache under the app sandbox. No network, no
    /// external URL, no upload, no connection to the running Windows game.
    private func cachedWorldURL() -> URL {
        let base = FileManager.default.urls(for:.applicationSupportDirectory,in:.userDomainMask)[0]
        return base.appendingPathComponent("owned-goodsprings.fnvworld")
    }

    private func openWorld(_ url: URL, persist: Bool = true) {
        guard !loading else { return }
        loading = true
        status = "Reading original Fallout: New Vegas cell geometry..."
        let installed = cachedWorldURL()
        // Do not block the UI while decoding 100+ original NIFs and textures.
        DispatchQueue.global(qos:.userInitiated).async {
            let result: Result<GoodspringsWorld,Error> =
                Result { try GoodspringsWorld.load(from:url) }
            var cacheWarning: String? = nil
            if persist, case .success = result {
                // Do not preserve a malformed or non-renderable package.
                // Copy to a private staging file and atomically replace the
                // old working world ONLY after the copy succeeds.
                do {
                    let fm=FileManager.default
                    try fm.createDirectory(at:installed.deletingLastPathComponent(),
                                           withIntermediateDirectories:true)
                    let staging=installed.deletingLastPathComponent()
                        .appendingPathComponent("world-stage-" + UUID().uuidString)
                    defer {try? fm.removeItem(at:staging)}
                    let granted=url.startAccessingSecurityScopedResource()
                    defer {if granted{url.stopAccessingSecurityScopedResource()}}
                    try fm.copyItem(at:url,to:staging)
                    if fm.fileExists(atPath:installed.path) {
                        _ = try fm.replaceItemAt(installed,withItemAt:staging)
                    } else {
                        try fm.moveItem(at:staging,to:installed)
                    }
                    var owned=installed
                    var attributes=URLResourceValues()
                    attributes.isExcludedFromBackup=true
                    try owned.setResourceValues(attributes)
                } catch {
                    cacheWarning=" (offline save failed: \(error.localizedDescription))"
                }
            }
            DispatchQueue.main.async {
                loading=false
                switch result {
                case .failure(let error):
                    status="Goodsprings world import failed: \(error.localizedDescription)"
                case .success(let world):
                    loadedWorld=world
                    sceneName=world.cellName
                    joystick=SIMD2<Float>(0,0)
                    status="Loaded \(world.instances.count) authored New Vegas references."
                        + (cacheWarning == nil ? " Offline world stored locally." : (cacheWarning ?? ""))
                }
            }
        }
    }
}

private struct MoveStick: View {
    @Binding var value: SIMD2<Float>
    @State private var offset: CGSize = .zero
    private let radius: CGFloat = 48

    var body: some View {
        ZStack {
            Circle().fill(.black.opacity(0.42))
                .overlay(Circle().stroke(.white.opacity(0.35),lineWidth:1))
                .frame(width:122,height:122)
            Circle().fill(.white.opacity(0.60))
                .frame(width:54,height:54)
                .offset(offset)
        }
        .frame(width:124,height:124)
        .contentShape(Circle())
        .gesture(DragGesture(minimumDistance:0)
            .onChanged { event in
                let x=event.translation.width
                let y=event.translation.height
                let mag=max(sqrt(x*x+y*y),1)
                let factor=min(1,radius/mag)
                let dx=x*factor,dy=y*factor
                offset=CGSize(width:dx,height:dy)
                // Up on the screen is forward movement in world space.
                value=SIMD2<Float>(Float(dx/radius),Float(-dy/radius))
            }
            .onEnded { _ in
                offset = .zero
                value = SIMD2<Float>(0,0)
            })
    }
}
