//
//  HotasRemapperLib.h
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

#ifndef HotasRemapperLib_h
#define HotasRemapperLib_h

#include <stdbool.h>
#include <stdio.h>

enum DeviceType {
  kJoystick = 0,
  kThrottle = 1,
  kVirtualDevice = 2,
};

void* OpenLib(void (*connection_status_callback)(enum DeviceType device_type,
                                                 bool is_connected));
void LoadInputRemapping(void* handle, const char* file_path_ptr);
void CloseLib(void* handle);

#endif /* HotasRemapperLib_h */
