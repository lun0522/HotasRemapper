# Feasibility of Using HOTAS in MSFS on Xbox Consoles

Do you want to use virtually any **joysticks** and **throttles** (maybe rudder pedals and other gaming devices as well) in **Microsoft Flight Simulator** (maybe other flight simulation games as well) on an Xbox console? This project is my personal experiment of:

1. Using a **Mac** to read input events from my joystick and throttle.
2. Remapping those events to standard keyboard events.
3. Sending them over Bluetooth to a **Raspberry Pi Zero 2W** board.
4. Making the Raspberry Pi act as a USB keyboard that can be recognized by an Xbox console.

Not all inputs can be easily remapped to keyboard events, but there might be workarounds. For example, the X- and Y-axis of the joystick couldn’t be remapped this way, but you can remap it to a thumbstick and feed the input events to Xbox via the Remote Play feature, which works on a Mac as well.

This is a pretty low-cost way to make my non-Xbox-licensed HOTAS work with the Xbox console with a help of a Mac, with a bit of programming in Rust and Swift. In this article, I will share what I found during my journey of discovery, such as related hardware products and programming tutorials. You may follow a similar path to make your gaming devices work in more games on the console.

# Motivation

It’s a simple story. I got my Thrustmaster joystick and throttle first, which work perfectly in MSFS 2020 on a Windows PC. However, I don’t own a very powerful PC so I had to live with poor resolutions and frame rates. As a casual gamer who don’t even play >3 games, I don’t plan to invest $1k+ on a gaming PC. Then I saw reviews mentioning MSFS works pretty well in 4k resolution on Xbox series X, so I gave it a try. The game looks stunning with the new setup, but unfortunately, I found that it only supports licensed accessories (i.e. those having an Xbox button) while my HOTAS isn’t one of it, and I can’t simply install a certain driver on the console to make my stuff supported as I would do on a normal PC.

I have limited options:

1. Just play the game with an Xbox controller or keyboard & mouse. The user experience might be the worst.
2. Use some software to allow my HOTAS to function as an Xbox controller. For example, [x360ce](https://www.x360ce.com) is capable of that on PC. However, a controller doesn’t have many buttons so I’ll probably not able to map all of my HOTAS buttons, and I’ll still need a way to send those events to the console, which isn’t easy at all.
3. Invest much more on a powerful gaming PC. Is it really worth it for casual gamers?
4. Buy an Xbox-licensed HOTAS. There are very limited choices on the market, and I’d be happier with the one I have.
5. Buy a third-party adapter that supports my HOTAS. There are a few adapters in the market, such as [Wingman XB](https://www.brookaccessory.com/detail/59327520/) and [Titan One](https://www.consoletuner.com/products/titan-one/), but they either explicitly don’t support my HOTAS, or don’t have any instructions/discussions on how to set it up in my case. Besides, they are not cheap.
6. Build an “adapter” by myself. Even though the Xbox console doesn’t support most of those gaming devices, it does support keyboard and mouse. What if I can convert the events generated by my HOTAS to keyboard and/or mouse events and feed them to the console over USB?

I have also noticed the Remote Play feature of the console. However, only standard gamepads are supported in that mode, meaning you still have to use Xbox controllers or other Xbox-licensed gamepads, so it doesn’t broaden my options.

So, I began my journey in looking for a low-cost way to make my HOTAS work on the console. I planned to start looking for related apps, and write some code to fill the gaps if necessary.

# Prior Art

What if there are already some apps that can achieve what I want, and I just don’t need to write any code? I have been doing my research mostly on Mac, and I found these interesting apps:

1. [Enjoyable](https://yukkurigames.com/enjoyable/). It is able to read input events from the HOTAS, identify which axis/button/hat switch they come from, and generate customizable mouse and keyboard events accordingly. The code is open-sourced. It work pretty nicely with my devices, but I can see a few problems:
    1. Not all inputs are supported, e.g. sliders.
    2. Cannot remap an axis to multiple buttons, which is necessary for remapping the throttle axis to keyboard buttons.
    3. Directly injects mouse and keyboard events to the operating system. This might be desired by some of their users but not me, since I actually want to route those to another device.
    4. The code was pretty old. I can no longer compile it with the latest MacOS and Xcode.
2. [KeyPad](https://bluetooth-keyboard.com/keypad/). It enables user to use their Mac as the Bluetooth trackpad and keyboard for their iPhones, iPads and so on. It is pretty easy to use according to my testing, but few problems arose:
    1. It works well between my Mac and iPad, but it doesn’t seem to be able to connect to a Raspberry Pi, which is definitely not a major use case of it.
    2. The latency seems a bit high when typing to an iPad, which is okay for typing but less ideal for gaming.
3. [Greenlight](https://github.com/unknownskl/greenlight). There is no Xbox app on Mac, so Remote Play isn’t officially supported here. This project brings that feature to Mac. It works pretty well despite that some issues on Github mentioned that the resolution is likely only 720p and there is no way to control it. It also maps some keyboard buttons to controller inputs ([mapping table](https://github.com/unknownskl/greenlight#keyboard-controls)). I found that even though there is no driver for my HOTAS on Mac, at least the joystick’s X- and Y-axis are actually working when I play MSFS through this app.

All the apps mentioned above are free to use. There are other paid apps with similar features, and there should be more similar apps on Windows. Note that Thrustmaster offers the [TARGET](https://ts.thrustmaster.com/download/accessories/pc/hotas/software/TARGET/TARGET_User_Manual_ENG.pdf) app that is capable of very complex remapping for HOTAS inputs, I haven’t tried that myself but I believe that is the toolbox desired by power users.

I haven’t found any app that directly satisfies all my needs, or several apps that I can simply put together to work, so I decided to reinvent some wheels.

# Project Deep Dive

## Choice of Hardware

Besides the HOTAS and Xbox console, the major hardware components involved are:

1. A computer, which reads HOTAS input events, remap them to keyboard events, and send them out wirelessly.
2. An external device, which receives keyboard events wirelessly and forwards them to the console via USB.

The computer can be a Mac/PC/Linux machine, as long as it can read USB inputs and send data wirelessly. I use a Mac in this project, but it shouldn’t be too hard to make the code cross-platform. In fact, there are more documentations and frameworks that can be used on Windows compared to MacOS.

Regarding the external device, it must be able to function as a USB device (like a real mouse, keyboard or USB flash drive), while almost all computers can only act as the USB host. The [Raspberry Pi 2 W](https://www.raspberrypi.com/products/raspberry-pi-zero-2-w/) caught my eye because it has two micro USB ports (one of which supports USB OTG so it can be a USB host) and a mini HDMI port, and it only costs $15. The [Raspberry Pi Pico](https://www.raspberrypi.com/products/raspberry-pi-pico/) is even cheaper (only $4) and also supports USB host mode. I went with the former for the sake of the HDMI port but the latter may also just work.

## Software Overview

On the software side, let’s break down the task:

1. Reading input events from the HOTAS using Apple’s [IOKit](https://developer.apple.com/documentation/iokit). Thanks for the [Enjoyable](https://github.com/shirosaki/enjoyable) project, I learnt that for free, although their code was written in Objective-C prior to 2016.
2. Remapping those events to keyboard events. This part allows the most customization. I’m using [protobuf](https://protobuf.dev) to keep it flexible.
3. Sending remapped events over Bluetooth using Apple’s [IOBluetooth](https://developer.apple.com/documentation/iobluetooth) framework.
4. Running Raspberry Pi as a USB device, and making it forward whatever it receives over Bluetooth to the USB port. Luckily there are several tutorials online since other folks also tried to use Raspberry Pi boards as keyboards. We’ll need to touch a bit of the [HID standard](https://en.wikipedia.org/wiki/Human_interface_device).

I prefer to use modern cross-platform languages, although lots of APIs used in this project are platform dependent. Besides, since it will handle inputs for gaming, we do want to minimize the overhead. As a result, I implemented most of things in Rust, and also use Swift when writing the UI for Mac and when using Apple’s frameworks that don’t have good Rust wrappers yet.