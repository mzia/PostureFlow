import Foundation
import PostureFlowShared

NSLog("[PostureFlowHelper] Starting com.postureflow.helper daemon...")

let listener = NSXPCListener(machServiceName: PostureFlowIPCConstants.machServiceName)
let service = HelperService.shared
listener.delegate = service

listener.resume()
NSLog("[PostureFlowHelper] Mach service listener running on '%@'", PostureFlowIPCConstants.machServiceName)

// Keep the daemon running in runloop
RunLoop.main.run()
