# Window Sizing & Universal Responsiveness

This document describes the window scaling, layout responsiveness, and aspect-ratio unlocking capabilities of the configurator.

---

## Free Window Resizing Toggle

By default, desktop UI frameworks impose strict minimum window boundaries (e.g. 700x500 pixels) to avoid component clipping. However, users on diverse monitor setups, tiling window managers (i3, Sway, Hyprland), or compact displays may require custom window shapes.

### How to Unlock Window Constraints:
1. Open the **App Settings** tab.
2. Locate the **Window Dimensions & Aspect Ratio Restrictions** section.
3. Check the box labeled **Unlock Window Sizing Restrictions (Allow Free Resizing Without Minimum Bounds)**.
4. The minimum inner boundary clamp is immediately reduced to `280x200` pixels, enabling completely free, arbitrary resizing.

---

## Quick Sizing Presets

For convenient one-click window positioning, three presets are provided:

- **Default Layout (`880 x 640 px`)**: Optimal viewport dimensions showing the full chassis diagram and parameter panels simultaneously without requiring horizontal scrolling.
- **Compact Layout (`720 x 520 px`)**: Streamlined footprint suitable for 1080p splitscreen or auxiliary displays.
- **Wide Studio Layout (`1120 x 740 px`)**: Expanded workspace ideal for multi-macro editing and hardware packet sniffing.

---

## HiDPI Scaling & Display Zoom

The configurator features an internal display zoom multiplier independent of OS scaling:

- **100%**: Native 1:1 pixel rendering for compact screens.
- **125% (Standard Default)**: Balanced UI sizing for standard 1080p/1440p monitors.
- **150%**: Enhanced legibility for high-resolution displays.
- **175%**: Comfortable viewing on 4K laptops.
- **200% (HiDPI)**: Full HiDPI mode with crisp vector scaling.

All layout panels utilize `egui::ScrollArea::both().auto_shrink([false, false])`, guaranteeing that UI elements are never clipped or truncated regardless of scale factor.
