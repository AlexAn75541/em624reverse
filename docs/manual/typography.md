# Cloud Fonts & Custom Typography

The EM624 configurator includes a **Dynamic Cloud Typography Engine** capable of querying, verifying, and activating fonts directly from public font repositories without requiring external server-side setups or OS font installations.

---

## Supported Font Repositories

The engine supports two primary freely available public repositories, plus arbitrary direct links:

1. **Google Fonts (1,500+ Fonts)**:
   - Covers popular open-source typefaces including *Inter, Roboto, Montserrat, Fira Code, Poppins, Open Sans, Nunito, Ubuntu*, and more.
2. **Fontshare (Free High-Quality Typography)**:
   - Professional design-forward typefaces developed by the Indian Type Foundry, including *General Sans, Satoshi, Clash Display, Cabinet Grotesk, Switzer, Cabinet*, etc.
3. **Direct TTF / OTF URL**:
   - Paste any valid raw `https://.../font.ttf` or `.otf` link to load custom corporate or private fonts.

---

## How to Verify and Apply a Cloud Font

1. Navigate to the **App Settings** tab.
2. Scroll to the **Typography & Cloud Font Customization** section.
3. Enter the font name (e.g., `Inter`, `Roboto`, `Satoshi`, `General Sans`, `Fira Code`) or click any of the **Quick Test Presets**.
4. Select your preferred **Font Source** (`Auto`, `Google Fonts`, or `Fontshare`).
5. Click **Verify Font**:
   - The application spawns a non-blocking background thread with a spinner.
   - It performs an HTTP request to query the repository's CSS manifest, parses the TrueType download URL, and inspects the binary header to verify valid font magic bytes (`0x00010000`, `OTTO`, `true`).
6. **State Indicator**:
   - If found and verified, a green confirmation badge appears displaying the font name, provider, and file size in KB.
   - The **Apply Font** button changes from greyed-out (disabled) to brightly highlighted.
   - If not found or spelled incorrectly, a clear error message is shown suggesting Title Case spelling corrections.
7. Click **Apply Font**:
   - The font is immediately cached to your data directory at `.fonts/<font_name>.ttf`.
   - The GUI context refreshes its font definition maps in real time without restarting the application!
   - CascadiaMono Nerd Font icons and Dosis-Bold fallbacks remain intact, ensuring no broken symbols or missing glyphs.
8. To revert to the factory look at any time, click **Reset to Default**.
