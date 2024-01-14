import IOBluetooth

public typealias ConnectionStatusCallback = @convention(c) (Bool) -> Void

// TODO: MAC address and RFCOMM channel id should be specified.
@_cdecl("OpenBluetoothLib")
public func openBluetoothLib(
  virtualDeviceCallback: ConnectionStatusCallback,
  rfcommChannelCallback: ConnectionStatusCallback
) {
  BluetoothManager.shared.start(
    with: virtualDeviceCallback, and: rfcommChannelCallback)
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
    with virtualDeviceCallback: ConnectionStatusCallback,
    and rfcommChannelCallback: ConnectionStatusCallback
  ) {
    print("Starting Bluetooth manager")
    isRunning = true
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
    if fromDevice.addressString != "b8-27-eb-c7-5b-1d" {
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
    rfcommChannel = RFCOMMChannel(device: fromDevice, delegate: self)
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
  private var channel: IOBluetoothRFCOMMChannel? {
    didSet {
      delegate.rfcommChannelConnectionDidChange(isConnected: channel != nil)
    }
  }

  init(
    device: IOBluetoothDevice,
    delegate: RFCOMMChannelConnectionDelegate
  ) {
    self.delegate = delegate
    super.init()

    print("Opening RFCOMM channel")
    var rfcommChannel: IOBluetoothRFCOMMChannel?
    device.openRFCOMMChannelAsync(
      &rfcommChannel,
      withChannelID: 1,
      delegate: self)
  }

  func send(buffer: UnsafePointer<CChar>, length: Int32) {
    if let channel = self.channel {
      let ret = channel.writeSync(
        UnsafeMutableRawPointer(mutating: buffer), length: UInt16(length))
      if ret != kIOReturnSuccess {
        print("Failed to write to RFCOMM channel:", ret)
      }
    }
  }

  func stop() {
    if let channel = self.channel {
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
      self.channel = channel
    } else {
      print("Failed to open RFCOMM channel:", error)
    }
  }

  func rfcommChannelClosed(_ channel: IOBluetoothRFCOMMChannel!) {
    if channel == self.channel {
      print("RFCOMM channel closed")
      self.channel = nil
    }
  }
}

private func toString(_ device: IOBluetoothDevice) -> String {
  """
  {device name: "\(device.name ?? "Unknown name")", \
  address: "\(device.addressString ?? "Unknown address")"}
  """
}
