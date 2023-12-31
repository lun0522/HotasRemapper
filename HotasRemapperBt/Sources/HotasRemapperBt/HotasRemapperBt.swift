import IOBluetooth

@_cdecl("open_bluetooth_lib")
public func openBluetoothLib() {
  BluetoothManager.shared.start()
}

@_cdecl("close_bluetooth_lib")
public func closeBluetoothLib() {
  BluetoothManager.shared.stop()
}

class BluetoothManager: NSObject {
  static let shared = BluetoothManager()

  var isRunning = false

  func start() {
    print("Starting Bluetooth manager")
    isRunning = true
    IOBluetoothDevice.register(
      forConnectNotifications: self,
      selector: #selector(didConnect(notification:fromDevice:)))
  }

  func stop() {
    print("Stopping Bluetooth manager")
    isRunning = false
  }

  @objc private func didConnect(
    notification: IOBluetoothUserNotification,
    fromDevice: IOBluetoothDevice
  ) {
    if !isRunning {
      notification.unregister()
      return
    }
    print("Bluetooth device connected:", toString(fromDevice))
    fromDevice.register(
      forDisconnectNotification: self,
      selector: #selector(didDisconnect(notification:fromDevice:)))
  }

  @objc private func didDisconnect(
    notification: IOBluetoothUserNotification,
    fromDevice: IOBluetoothDevice
  ) {
    if !isRunning {
      notification.unregister()
      return
    }
    print("Bluetooth device disconnected", toString(fromDevice))
  }
}

private func toString(_ device: IOBluetoothDevice) -> String {
  """
  {device name: "\(device.name ?? "Unknown name")", \
  address: \(device.addressString ?? "Unknown address")}
  """
}
