import IOBluetooth
import SwiftRs

public typealias ConnectionStatusCallback = @convention(c) (Bool) -> Void

/// The format of `hostMacAddress` must be "xx-xx-xx-xx-xx-xx".
@_cdecl("OpenBluetoothLib")
public func openBluetoothLib(
  hostMacAddress: SRString,
  rfcommChannelId: UInt8,
  virtualDeviceCallback: ConnectionStatusCallback,
  rfcommChannelCallback: ConnectionStatusCallback
) {
  BluetoothManager.shared.start(
    hostMacAddress: hostMacAddress,
    rfcommChannelId: rfcommChannelId,
    virtualDeviceCallback: virtualDeviceCallback,
    rfcommChannelCallback: rfcommChannelCallback)
}

@_cdecl("SendDataViaBluetooth")
public func sendDataViaBluetooth(buffer: UnsafePointer<CChar>, length: Int32) {
  BluetoothManager.shared.send(buffer: buffer, length: length)
}

@_cdecl("CloseBluetoothLib")
public func closeBluetoothLib() {
  BluetoothManager.shared.stop()
}

class BluetoothManager: NSObject, RFCOMMChannelConnectionDelegate {
  static let shared = BluetoothManager()

  private var hostMacAddress = ""
  private var rfcommChannelId: UInt8 = 0
  private var virtualDeviceCallback: ConnectionStatusCallback?
  private var rfcommChannelCallback: ConnectionStatusCallback?
  private var isRunning = false
  private var isVirtualDeviceConnected = false {
    didSet {
      virtualDeviceCallback?(isVirtualDeviceConnected)
    }
  }
  private var rfcommChannel: RFCOMMChannel?

  func start(
    hostMacAddress: SRString,
    rfcommChannelId: UInt8,
    virtualDeviceCallback: ConnectionStatusCallback,
    rfcommChannelCallback: ConnectionStatusCallback
  ) {
    print("Starting Bluetooth manager")
    isRunning = true
    self.hostMacAddress = hostMacAddress.toString()
    self.rfcommChannelId = rfcommChannelId
    self.virtualDeviceCallback = virtualDeviceCallback
    self.rfcommChannelCallback = rfcommChannelCallback
    IOBluetoothDevice.register(
      forConnectNotifications: self,
      selector: #selector(didConnect(notification:fromDevice:)))
  }

  func send(buffer: UnsafePointer<CChar>, length: Int32) {
    rfcommChannel?.send(buffer: buffer, length: length)
  }

  func stop() {
    print("Stopping Bluetooth manager")
    isRunning = false
    rfcommChannel?.stop()
  }

  // MARK: - IOBluetoothDevice

  @objc private func didConnect(
    notification: IOBluetoothUserNotification,
    fromDevice: IOBluetoothDevice
  ) {
    if !isRunning {
      notification.unregister()
      return
    }

    let deviceInfo = toString(fromDevice);
    if fromDevice.addressString != hostMacAddress {
      print("Ignoring Bluetooth device:", deviceInfo)
      return
    }

    print("Found virtual device via Bluetooth:", deviceInfo)
    // We may get notified more than once.
    if isVirtualDeviceConnected {
      return
    }
    isVirtualDeviceConnected = true
    fromDevice.register(
      forDisconnectNotification: self,
      selector: #selector(didDisconnect(notification:fromDevice:)))
    rfcommChannel = RFCOMMChannel(
      device: fromDevice,
      channelId: rfcommChannelId,
      delegate: self)
  }

  @objc private func didDisconnect(
    notification: IOBluetoothUserNotification,
    fromDevice: IOBluetoothDevice
  ) {
    print("Virtual device disconnected")
    isVirtualDeviceConnected = false
    if !isRunning {
      notification.unregister()
    }
  }

  // MARK: - RFCOMMChannelConnectionDelegate

  func rfcommChannelConnectionDidChange(isConnected: Bool) {
    rfcommChannelCallback?(isConnected)
  }
}

private protocol RFCOMMChannelConnectionDelegate {
  func rfcommChannelConnectionDidChange(isConnected: Bool)
}

private class RFCOMMChannel: NSObject, IOBluetoothRFCOMMChannelDelegate {
  private let delegate: RFCOMMChannelConnectionDelegate
  private var channel: IOBluetoothRFCOMMChannel?
  private var isConnected = false {
    didSet {
      delegate.rfcommChannelConnectionDidChange(isConnected: isConnected)
    }
  }

  init(
    device: IOBluetoothDevice,
    channelId: UInt8,
    delegate: RFCOMMChannelConnectionDelegate
  ) {
    self.delegate = delegate
    super.init()

    print("Opening RFCOMM channel")
    device.openRFCOMMChannelAsync(
      &channel,
      withChannelID: channelId,
      delegate: self)
  }

  func send(buffer: UnsafePointer<CChar>, length: Int32) {
    if isConnected, let channel = self.channel {
      let ret = channel.writeSync(
        UnsafeMutableRawPointer(mutating: buffer), length: UInt16(length))
      if ret != kIOReturnSuccess {
        print("Failed to write to RFCOMM channel:", ret)
      }
    }
  }

  func stop() {
    if isConnected, let channel = self.channel {
      print("Closing RFCOMM channel")
      channel.close()
    }
  }

  // MARK: - IOBluetoothRFCOMMChannelDelegate

  func rfcommChannelOpenComplete(
    _ channel: IOBluetoothRFCOMMChannel!,
    status error: IOReturn
  ) {
    if error == kIOReturnSuccess {
      print("Opened RFCOMM channel")
      isConnected = true
    } else {
      print("Failed to open RFCOMM channel:", error)
    }
  }

  func rfcommChannelClosed(_ channel: IOBluetoothRFCOMMChannel!) {
    print("RFCOMM channel closed")
    isConnected = false
  }
}

private func toString(_ device: IOBluetoothDevice) -> String {
  """
  {device name: "\(device.name ?? "Unknown name")", \
  address: "\(device.addressString ?? "Unknown address")"}
  """
}
