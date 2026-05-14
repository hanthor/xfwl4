use libdisplay_info::{
    cta::{self, CTA},
    cvt,
    displayid::{self, DisplayId},
    dmt,
    edid::{
        self, ColorPoint, CvtAspectRatio, CvtScaling, CvtTimingCode, CvtTimingCodePreferredVrate,
        DetailedTimingDef, DetailedTimingDefSignalType, DisplayDescriptorRef,
        DisplayRangeLimitsType, Edid, EstablishedTimings, ExtensionRef, StandardTimingRef,
    },
    gtf,
    info::Info,
};

pub fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("no path specified");
        return Ok(());
    };

    let blob = std::fs::read(path)?;
    let info = Info::parse_edid(&blob)?;

    if let Some(edid) = info.edid() {
        print_edid(&edid);

        let extensions = edid.extensions().len();
        if extensions > 0 {
            println!("  Extension blocks: {}", extensions);
        }

        for (index, ext) in edid.extensions().iter().enumerate() {
            print_ext(ext, index);
        }
    }

    print!("\n----------------\n\n");

    if let Some(failure_msg) = info.failure_msg() {
        println!("{:?}\n", failure_msg);
        println!("EDID conformity: FAIL");
    } else {
        println!("EDID conformity: PASS");
    }

    Ok(())
}

fn print_edid(edid: &Edid<'_>) {
    println!("Block 0, Base EDID:");
    println!(
        "  EDID Structure Version & Revision: {}.{}",
        edid.version(),
        edid.revision()
    );

    let vendor_product = edid.vendor_product();
    println!("  Vendor & Product Identification:");
    println!("    Manufacturer: {:?}", vendor_product.manufacturer);
    println!("    Model: {}", vendor_product.product);
    if let Some(serial) = vendor_product.serial {
        println!("    Serial Number: {}", serial);
    }
    if let Some(model_year) = vendor_product.model_year {
        println!("    Model year: {}", model_year);
    } else {
        println!(
            "    Made in: week {} of {}",
            vendor_product.manufacture_week, vendor_product.manufacture_year
        );
    }

    println!("  Basic Display Parameters & Features:");
    if let Some(video_input_analog) = edid.video_input_analog() {
        println!("    Analog display");
        println!(
            "    Signal Level Standard: {:?}",
            video_input_analog.signal_level_std
        );

        match video_input_analog.video_setup {
            edid::VideoInputAnalogVideoSetup::BlankLevelEqBlack => {
                println!("    Blank level equals black level")
            }
            edid::VideoInputAnalogVideoSetup::BlankToBlackSetupPedestal => {
                println!("    Blank-to-black setup/pedestal")
            }
        };
        print!("    Sync:");
        if video_input_analog.sync_separate {
            print!(" Separate");
        }
        if video_input_analog.sync_composite {
            print!(" Composite");
        }
        if video_input_analog.sync_on_green {
            print!(" SyncOnGreen");
        }
        if video_input_analog.sync_serrations {
            print!(" Serration");
        }
        println!();
    }
    let video_input_digital = edid.video_input_digital();
    if let Some(video_input_digital) = video_input_digital.as_ref() {
        println!("    Digital display");
        if edid.revision() >= 4 {
            if let Some(color_bit_depth) = video_input_digital.color_bit_depth {
                println!("    Bits per primary color channel: {}", color_bit_depth);
            } else {
                println!("    Color depth is undefined");
            }
            println!("    {:?}\n", video_input_digital.interface);
        }
        if video_input_digital.dfp1 {
            println!("    DFP 1.x compatible TMDS");
        }
    }
    let screen_size = edid.screen_size();
    if let (Some(width_cm), Some(height_cm)) = (screen_size.width_cm, screen_size.height_cm) {
        println!("    Maximum image size: {} cm x {} cm", width_cm, height_cm);
    } else if let Some(aspect_ration) = screen_size.landscape_aspect_ratio {
        println!("    Aspect ratio: {:2} (landscape)", aspect_ration);
    } else if let Some(portait_aspect_ratio) = screen_size.portait_aspect_ratio {
        println!("    Aspect ratio: {:2} (portrait)", portait_aspect_ratio);
    } else {
        println!("    Image size is variable");
    }

    if let Some(gamma) = edid.basic_gamma() {
        println!("    Gamma: {}", gamma);
    } else {
        println!("    Gamma is defined in an extension block");
    }

    let dpms = edid.dpms();
    if dpms.standby || dpms.suspend || dpms.off {
        print!("    DPMS levels:");
        if dpms.standby {
            print!(" Standby");
        }
        if dpms.suspend {
            print!(" Suspend");
        }
        if dpms.off {
            print!(" Off");
        }
        println!();
    }

    if video_input_digital.is_none() || edid.revision() < 4 {
        println!("    {:?}", edid.display_color_type());
    }

    if let Some(color_encoding_formats) = edid.color_encoding_formats() {
        assert!(color_encoding_formats.rgb444);
        print!("    Supported color formats: RGB 4:4:4");
        if color_encoding_formats.ycrcb444 {
            print!(", YCrCb 4:4:4");
        }
        if color_encoding_formats.ycrcb422 {
            print!(", YCrCb 4:2:2");
        }
        println!();
    }

    let misc_features = edid.misc_features();
    if misc_features.srgb_is_primary {
        println!("    Default (sRGB) color space is primary color space");
    }
    if edid.revision() >= 4 {
        assert!(misc_features.has_preferred_timing);
        if misc_features.preferred_timing_is_native {
            println!("    First detailed timing includes the native pixel format and preferred refresh rate");
        } else {
            println!("    First detailed timing does not include the native pixel format and preferred refresh rate");
        }
    } else if misc_features.has_preferred_timing {
        println!("    First detailed timing is the preferred timing");
    }
    if misc_features.continuous_freq {
        println!("    Display is continuous frequency");
    }
    if misc_features.default_gtf {
        println!("    Supports GTF timings within operating range");
    }

    // /* edid-decode truncates the result, but %f rounds it */
    // chromaticity_coords = di_edid_get_chromaticity_coords(edid);
    // print!("  Color Characteristics:\n");
    // print!("    Red  : %.4f, %.4f\n",
    //        truncate_chromaticity_coord(chromaticity_coords->red_x),
    //        truncate_chromaticity_coord(chromaticity_coords->red_y));
    // print!("    Green: %.4f, %.4f\n",
    //        truncate_chromaticity_coord(chromaticity_coords->green_x),
    //        truncate_chromaticity_coord(chromaticity_coords->green_y));
    // print!("    Blue : %.4f, %.4f\n",
    //        truncate_chromaticity_coord(chromaticity_coords->blue_x),
    //        truncate_chromaticity_coord(chromaticity_coords->blue_y));
    // print!("    White: %.4f, %.4f\n",
    //        truncate_chromaticity_coord(chromaticity_coords->white_x),
    //        truncate_chromaticity_coord(chromaticity_coords->white_y));

    print!("  Established Timings I & II:");
    let established_timings_i_ii = edid.established_timings();
    if !has_established_timings(&established_timings_i_ii) {
        print!(" none");
    }
    println!();
    if established_timings_i_ii.has_720x400_70hz {
        println!("    IBM     :   720x400    70.081663 Hz   9:5     31.467 kHz     28.320000 MHz");
    }
    if established_timings_i_ii.has_720x400_88hz {
        println!("    IBM     :   720x400    87.849542 Hz   9:5     39.444 kHz     35.500000 MHz");
    }
    if established_timings_i_ii.has_640x480_60hz {
        println!("    DMT 0x04:   640x480    59.940476 Hz   4:3     31.469 kHz     25.175000 MHz");
    }
    if established_timings_i_ii.has_640x480_67hz {
        println!("    Apple   :   640x480    66.666667 Hz   4:3     35.000 kHz     30.240000 MHz");
    }
    if established_timings_i_ii.has_640x480_72hz {
        println!("    DMT 0x05:   640x480    72.808802 Hz   4:3     37.861 kHz     31.500000 MHz");
    }
    if established_timings_i_ii.has_640x480_75hz {
        println!("    DMT 0x06:   640x480    75.000000 Hz   4:3     37.500 kHz     31.500000 MHz");
    }
    if established_timings_i_ii.has_800x600_56hz {
        println!("    DMT 0x08:   800x600    56.250000 Hz   4:3     35.156 kHz     36.000000 MHz");
    }
    if established_timings_i_ii.has_800x600_60hz {
        println!("    DMT 0x09:   800x600    60.316541 Hz   4:3     37.879 kHz     40.000000 MHz");
    }
    if established_timings_i_ii.has_800x600_72hz {
        println!("    DMT 0x0a:   800x600    72.187572 Hz   4:3     48.077 kHz     50.000000 MHz");
    }
    if established_timings_i_ii.has_800x600_75hz {
        println!("    DMT 0x0b:   800x600    75.000000 Hz   4:3     46.875 kHz     49.500000 MHz");
    }
    if established_timings_i_ii.has_832x624_75hz {
        println!("    Apple   :   832x624    74.551266 Hz   4:3     49.726 kHz     57.284000 MHz");
    }
    if established_timings_i_ii.has_1024x768_87hz_interlaced {
        println!("    DMT 0x0f:  1024x768i   86.957532 Hz   4:3     35.522 kHz     44.900000 MHz");
    }
    if established_timings_i_ii.has_1024x768_60hz {
        println!("    DMT 0x10:  1024x768    60.003840 Hz   4:3     48.363 kHz     65.000000 MHz");
    }
    if established_timings_i_ii.has_1024x768_70hz {
        println!("    DMT 0x11:  1024x768    70.069359 Hz   4:3     56.476 kHz     75.000000 MHz");
    }
    if established_timings_i_ii.has_1024x768_75hz {
        println!("    DMT 0x12:  1024x768    75.028582 Hz   4:3     60.023 kHz     78.750000 MHz");
    }
    if established_timings_i_ii.has_1280x1024_75hz {
        println!("    DMT 0x24:  1280x1024   75.024675 Hz   5:4     79.976 kHz    135.000000 MHz");
    }
    if established_timings_i_ii.has_1152x870_75hz {
        println!("    Apple   :  1152x870    75.061550 Hz 192:145   68.681 kHz    100.000000 MHz");
    }

    print!("  Standard Timings:");
    if edid.standard_timings().is_empty() {
        print!(" none");
    }
    println!();
    for standard_timing in edid.standard_timings() {
        print_standard_timing(standard_timing);
    }

    println!("  Detailed Timing Descriptors:\n");
    for (index, detailed_timing_def) in edid.detailed_timing_defs().enumerate() {
        print_detailed_timing_def(index, detailed_timing_def)
    }

    for display_descriptor in edid.display_descriptors() {
        print_display_desc(edid, display_descriptor);
    }
}

fn print_standard_timing(timing_ref: &StandardTimingRef) {
    let vert_video = timing_ref.vert_video();
    let dmt = timing_ref.dmt();
    let timing = timing_ref.inner();

    print!("    ");
    let (refresh, horiz_freq_hz, pixel_clock_mhz) = if let Some(dmt) = dmt {
        let hbl = dmt.horiz_blank - 2 * dmt.horiz_border;
        let vbl = dmt.vert_blank - 2 * dmt.vert_border;
        let horiz_total = dmt.horiz_video + hbl;
        let vert_total = dmt.vert_video + vbl;
        let refresh = dmt.pixel_clock_hz as f64 / (horiz_total * vert_total) as f64;
        let horiz_freq_hz = dmt.pixel_clock_hz as f64 / horiz_total as f64;
        let pixel_clock_mhz = dmt.pixel_clock_hz as f64 / (1000 * 1000) as f64;

        print!("DMT {:02x}", dmt.dmt_id);

        (refresh, horiz_freq_hz, pixel_clock_mhz)
    } else {
        /* TODO: CVT timings */

        let gtf_options = gtf::Options {
            h_pixels: timing.horiz_video,
            v_lines: vert_video,
            margins_rqd: false,
            ip_param: gtf::IpParam::VFrameRate,
            ip_freq_rqd: timing.refresh_rate_hz as f64,
            int_rqd: false,
            m: gtf::DEFAULT_M,
            c: gtf::DEFAULT_C,
            k: gtf::DEFAULT_K,
            j: gtf::DEFAULT_J,
        };
        let gtf = gtf::Timing::compute(gtf_options);

        let hbl = gtf.h_front_porch + gtf.h_sync + gtf.h_back_porch + 2 * gtf.h_border;
        let vbl = gtf.v_front_porch + gtf.v_sync + gtf.v_back_porch + 2 * gtf.v_border;
        let horiz_total = gtf.h_pixels + hbl;
        let vert_total = gtf.v_lines + vbl;
        /* Upstream edid-decode rounds the pixel clock to kHz... */
        let pixel_clock_khz = f64::round(gtf.pixel_freq_mhz * 1000f64);
        let refresh = (pixel_clock_khz * 1000f64) / (horiz_total * vert_total) as f64;
        let horiz_freq_hz = (pixel_clock_khz * 1000f64) / horiz_total as f64;
        let pixel_clock_mhz = pixel_clock_khz / 1000f64;

        print!("GTF     ");

        (refresh, horiz_freq_hz, pixel_clock_mhz)
    };

    print!(":");
    print!(" {:5}x{:-5}", timing.horiz_video, vert_video);
    print!(" {:10.6} Hz", refresh);
    print!("  {:?} ", timing.aspect_ratio);
    print!(
        " {:8.3} kHz {:13.6} MHz",
        horiz_freq_hz / 1000f64,
        pixel_clock_mhz
    );
    if dmt.map(|dmt| dmt.reduced_blanking).unwrap_or(false) {
        print!(" (RB)");
    }
    println!();
}

fn print_detailed_timing_def(index: usize, def: DetailedTimingDef) {
    let hbl = def.horiz_blank - 2 * def.horiz_border.unwrap_or_default();
    let vbl = def.vert_blank - 2 * def.vert_border.unwrap_or_default();
    let horiz_total = def.horiz_video + hbl;
    let vert_total = def.vert_video + vbl;
    let refresh = def.pixel_clock_hz as f64 / (horiz_total * vert_total) as f64;
    let horiz_freq_hz = def.pixel_clock_hz as f64 / horiz_total as f64;

    // compute_aspect_ratio(def.horiz_video, def.vert_video,
    // 		     &horiz_ratio, &vert_ratio);

    // signal_type_name = detailed_timing_def_signal_type_name(def.signal_type);
    // if signal_type_name != NULL) {
    // 	flags[flags_len++] = signal_type_name;
    // }
    // if detailed_timing_def_sync_serrations(def)) {
    // 	flags[flags_len++] = "serrate";
    // }
    // if detailed_timing_def_sync_on_green(def)) {
    // 	flags[flags_len++] = "sync-on-green";
    // }
    // if def.stereo != DI_EDID_DETAILED_TIMING_DEF_STEREO_NONE) {
    // 	flags[flags_len++] = detailed_timing_def_stereo_name(def.stereo);
    // }
    // if def.horiz_image_mm != 0 || def.vert_image_mm != 0) {
    // 	snprint!(size_mm, sizeof(size_mm), "%d mm x %d mm",
    // 		 def.horiz_image_mm, def.vert_image_mm);
    // 	flags[flags_len++] = size_mm;
    // }
    // assert(flags_len < sizeof(flags) / sizeof(flags[0]));

    print!("    DTD {}:", index);
    print!(" {:5}x{:-5}", def.horiz_video, def.vert_video);
    if def.interlaced {
        print!("i");
    }
    print!(" {:10.6} Hz", refresh);
    //print!(" {}:{}", horiz_ratio, vert_ratio);
    print!(
        " {:8.3} kHz {:13.6} MHz",
        horiz_freq_hz / 1000f64,
        def.pixel_clock_hz as f64 / (1000f64 * 1000f64)
    );
    // if flags_len > 0) {
    // 	char *flags_str = join_str(flags);
    // 	print!(" (%s)", flags_str);
    // 	free(flags_str);
    // }
    println!();

    let horiz_back_porch = hbl - def.horiz_sync_pulse - def.horiz_front_porch;
    print!(
        "                 Hfront {:4} Hsync {:3} Hback {:4}",
        def.horiz_front_porch, def.horiz_sync_pulse, horiz_back_porch
    );
    if let Some(horiz_border) = def.horiz_border {
        print!(" Hborder {}", horiz_border);
    }
    if def.signal_type == DetailedTimingDefSignalType::DigitalComposite {
        print!(
            " Hpol {:?}",
            def.digital_composite.unwrap().sync_horiz_polarity
        );
    } else if def.signal_type == DetailedTimingDefSignalType::DigitalSeparate {
        print!(
            " Hpol {:?}",
            def.digital_separate.unwrap().sync_horiz_polarity
        );
    }
    println!();

    let vert_back_porch = vbl - def.vert_sync_pulse - def.vert_front_porch;
    print!(
        "                 Vfront {:4} Vsync {:3} Vback {:4}",
        def.vert_front_porch, def.vert_sync_pulse, vert_back_porch
    );
    if let Some(vert_border) = def.vert_border {
        print!(" Vborder {}", vert_border);
    }
    if def.signal_type == DetailedTimingDefSignalType::DigitalSeparate {
        print!(
            " Vpol {:?}",
            def.digital_separate.unwrap().sync_vert_polarity
        );
    }
    println!();
}

fn print_display_desc(edid: &Edid<'_>, desc_ref: &DisplayDescriptorRef) {
    let tag = desc_ref.tag();

    print!("    {:?}:", tag);

    match tag {
        edid::DisplayDescriptorTag::ProductSerial
        | edid::DisplayDescriptorTag::DataString
        | edid::DisplayDescriptorTag::ProductName => {
            println!(" '{}'", desc_ref.string().unwrap());
        }
        edid::DisplayDescriptorTag::RangeLimits => {
            let range_limits = desc_ref.range_limits().unwrap();

            let mut range_limits_type = range_limits.type_;
            if edid.revision() < 4 && range_limits.type_ == DisplayRangeLimitsType::Bare {
                /* edid-decode always prints "GTF" for EDID 1.3 and
                 * earlier even if the display doesn't support it */
                range_limits_type = DisplayRangeLimitsType::DefaultGtf;
            }

            print!(
                "\n      Monitor ranges ({:?}): {}-{} Hz V, {}-{} kHz H",
                range_limits_type,
                range_limits.min_vert_rate_hz,
                range_limits.max_vert_rate_hz,
                range_limits.min_horiz_rate_hz / 1000,
                range_limits.max_horiz_rate_hz / 1000
            );
            if let Some(max_pixel_clock_hz) = range_limits.max_pixel_clock_hz {
                print!(", max dotclock {} MHz", max_pixel_clock_hz / (1000 * 1000));
            }
            println!();

            match range_limits_type {
                DisplayRangeLimitsType::SecondaryGtf => {
                    let secondary_gtf = range_limits.secondary_gtf.unwrap();
                    println!("      GTF Secondary Curve Block:");
                    println!(
                        "        Start frequency: {} kHz",
                        secondary_gtf.start_freq_hz / 1000
                    );
                    println!("        C: {:.1}%", secondary_gtf.c);
                    println!("        M: {}%/kHz", secondary_gtf.m);
                    println!("        K: {}", secondary_gtf.k);
                    println!("        J: {:.1}%", secondary_gtf.j);
                }
                DisplayRangeLimitsType::Cvt => {
                    let cvt = range_limits.cvt.unwrap();
                    println!("      CVT version {}.{}", cvt.version, cvt.revision);

                    if let Some(max_horiz_px) = cvt.max_horiz_px {
                        println!("      Max active pixels per line: {}", max_horiz_px);
                    }

                    print!("      Supported aspect ratios:");
                    if cvt.supported_aspect_ratio.contains(CvtAspectRatio::_4_3) {
                        print!(" 4:3");
                    }
                    if cvt.supported_aspect_ratio.contains(CvtAspectRatio::_16_9) {
                        print!(" 16:9");
                    }
                    if cvt.supported_aspect_ratio.contains(CvtAspectRatio::_16_10) {
                        print!(" 16:10");
                    }
                    if cvt.supported_aspect_ratio.contains(CvtAspectRatio::_5_4) {
                        print!(" 5:4");
                    }
                    if cvt.supported_aspect_ratio.contains(CvtAspectRatio::_15_9) {
                        print!(" 15:9");
                    }
                    println!();

                    println!(
                        "      Preferred aspect ratio: {:?}",
                        cvt.preferred_aspect_ratio
                    );

                    if cvt.standard_blanking {
                        println!("      Supports CVT standard blanking");
                    }
                    if cvt.reduced_blanking {
                        println!("      Supports CVT reduced blanking");
                    }

                    if !cvt.supported_scaling.is_empty() {
                        println!("      Supported display scaling:");
                        if cvt.supported_scaling.contains(CvtScaling::HorizShrink) {
                            println!("        Horizontal shrink");
                        }
                        if cvt.supported_scaling.contains(CvtScaling::HorizStretch) {
                            println!("        Horizontal stretch");
                        }
                        if cvt.supported_scaling.contains(CvtScaling::VertShrink) {
                            println!("        Vertical shrink");
                        }
                        if cvt.supported_scaling.contains(CvtScaling::VertStretch) {
                            println!("        Vertical stretch");
                        }
                    }

                    println!(
                        "      Preferred vertical refresh: {} Hz",
                        cvt.preferred_vert_refresh_hz
                    );
                }
                _ => {}
            };
        }
        edid::DisplayDescriptorTag::StdTimingIds => {
            let standard_timing = desc_ref.standard_timings().unwrap();

            println!();
            for timing in standard_timing {
                print_standard_timing(timing);
            }
        }
        edid::DisplayDescriptorTag::ColorPoint => {
            for color_point in desc_ref.color_points() {
                print!("      ");
                print_color_point(color_point);
            }
        }
        edid::DisplayDescriptorTag::EstablishedTimingsIII => {
            println!();
            for timing in desc_ref.established_timings_iii() {
                print_dmt_timing(timing);
            }
        }
        edid::DisplayDescriptorTag::DcmData => {
            let color_management_data = desc_ref.color_management_data().unwrap();

            println!("      Version : {}", color_management_data.version);
            println!("      Red a3  : {:.2}", color_management_data.red_a3);
            println!("      Red a2  : {:.2}", color_management_data.red_a2);
            println!("      Green a3: {:.2}", color_management_data.green_a3);
            println!("      Green a2: {:.2}", color_management_data.green_a2);
            println!("      Blue a3 : {:.2}", color_management_data.blue_a3);
            println!("      Blue a2 : {:.2}", color_management_data.blue_a2);
        }
        edid::DisplayDescriptorTag::CvtTimingCodes => {
            println!();

            for timing_code in desc_ref.cvt_timing_codes() {
                print_cvt_timing_code(timing_code);
            }
        }
        _ => println!(),
    };
}

fn print_color_point(c: ColorPoint) {
    print!(
        "Index: {} White: {:.4}, {:.4} ",
        c.index, c.white_x, c.white_y
    );

    if let Some(gamma) = c.gamma {
        println!("Gamma: {:.2}", gamma);
    } else {
        println!("Gamma: is defined in an extension block");
    }
}

fn print_dmt_timing(t: dmt::Timing) {
    // int hbl, vbl, horiz_total, vert_total, horiz_ratio, vert_ratio;
    // double refresh, horiz_freq_hz, pixel_clock_mhz;

    // compute_aspect_ratio(t.horiz_video, t.vert_video,
    // 		     &horiz_ratio, &vert_ratio);

    let hbl = t.horiz_blank - 2 * t.horiz_border;
    let vbl = t.vert_blank - 2 * t.vert_border;
    let horiz_total = t.horiz_video + hbl;
    let vert_total = t.vert_video + vbl;
    let refresh = t.pixel_clock_hz as f64 / (horiz_total * vert_total) as f64;
    let horiz_freq_hz = t.pixel_clock_hz / horiz_total;
    let pixel_clock_mhz = t.pixel_clock_hz as f64 / (1000f64 * 1000f64);

    print!("      DMT {:02}:", t.dmt_id);
    print!(" {:5}x{:-5}", t.horiz_video, t.vert_video);
    print!(" {:10.6} Hz", refresh);
    //print!(" {:3}:{:-3}", horiz_ratio, vert_ratio);
    print!(
        " {:8.3} kHz {:13.6} MHz",
        horiz_freq_hz / 1000,
        pixel_clock_mhz
    );
    if t.reduced_blanking {
        print!(" (RB)");
    }
    println!();
}

fn print_cvt_timing_code(t: CvtTimingCode) {
    let mut options = cvt::Options {
        red_blank_ver: cvt::ReducedBlankingVersion::None,
        h_pixels: 0,
        v_lines: t.addressable_lines_per_field,
        ip_freq_rqd: 0f64,
        video_opt: false,
        vblank: 0f64,
        additional_hblank: 0,
        early_vsync_rqd: false,
        int_rqd: false,
        margins_rqd: false,
    };

    let (hratio, vratio) = match t.aspect_ratio {
        edid::CvtTimingCodeAspectRatio::_4_3 => (4, 3),
        edid::CvtTimingCodeAspectRatio::_16_9 => (16, 9),
        edid::CvtTimingCodeAspectRatio::_16_10 => (16, 10),
        edid::CvtTimingCodeAspectRatio::_15_9 => (15, 9),
    };

    options.h_pixels = 8 * (((options.v_lines * hratio) / vratio) / 8);

    if t.supports_50hz_sb {
        options.ip_freq_rqd = 50f64;
        options.red_blank_ver = cvt::ReducedBlankingVersion::None;

        let timing = cvt::Timing::compute(options);
        print_cvt_timing(
            timing,
            &options,
            hratio,
            vratio,
            t.preferred_vertical_rate == CvtTimingCodePreferredVrate::_50HZ,
            false,
        );
    }
    if t.supports_60hz_sb {
        options.ip_freq_rqd = 60f64;
        options.red_blank_ver = cvt::ReducedBlankingVersion::None;

        let timing = cvt::Timing::compute(options);
        print_cvt_timing(
            timing,
            &options,
            hratio,
            vratio,
            t.preferred_vertical_rate == CvtTimingCodePreferredVrate::_60HZ && !t.supports_60hz_rb,
            false,
        );
    }
    if t.supports_75hz_sb {
        options.ip_freq_rqd = 75f64;
        options.red_blank_ver = cvt::ReducedBlankingVersion::None;

        let timing = cvt::Timing::compute(options);
        print_cvt_timing(
            timing,
            &options,
            hratio,
            vratio,
            t.preferred_vertical_rate == CvtTimingCodePreferredVrate::_75HZ,
            false,
        );
    }
    if t.supports_85hz_sb {
        options.ip_freq_rqd = 85f64;
        options.red_blank_ver = cvt::ReducedBlankingVersion::None;

        let timing = cvt::Timing::compute(options);
        print_cvt_timing(
            timing,
            &options,
            hratio,
            vratio,
            t.preferred_vertical_rate == CvtTimingCodePreferredVrate::_85HZ,
            false,
        );
    }
    if t.supports_60hz_rb {
        options.ip_freq_rqd = 60f64;
        options.red_blank_ver = cvt::ReducedBlankingVersion::V1;

        let timing = cvt::Timing::compute(options);
        print_cvt_timing(
            timing,
            &options,
            hratio,
            vratio,
            t.preferred_vertical_rate == CvtTimingCodePreferredVrate::_60HZ,
            true,
        );
    }
}

fn print_cvt_timing(
    t: cvt::Timing,
    options: &cvt::Options,
    hratio: i32,
    vratio: i32,
    preferred: bool,
    rb: bool,
) {
    let hbl = t.h_front_porch + t.h_sync + t.h_back_porch;
    let htotal = t.total_active_pixels + hbl;

    print!("      CVT: {:5}x{:-5}", { options.h_pixels }, {
        options.v_lines
    });
    print!(" {:10.6} Hz", t.act_frame_rate);
    print!(" {:3}:{:-3}", hratio, vratio);
    print!(
        " {:8.3} kHz {:13.6} MHz",
        t.act_pixel_freq * 1000f64 / htotal,
        { t.act_pixel_freq }
    );

    if preferred && rb {
        print!(" (RB, preferred vertical rate)");
    } else if preferred {
        print!(" (preferred vertical rate)");
    } else if rb {
        print!(" (RB)");
    }

    println!();
}

fn print_ext(ext: &ExtensionRef, ext_index: usize) {
    let tag = ext.tag();

    println!("\n----------------\n");
    print!("Block {}, {:?}:", ext_index + 1, tag);

    match tag {
        edid::ExtensionTag::CEA => {
            print_cta(CTA::from_extension(ext).unwrap());
        }
        edid::ExtensionTag::DisplayId => {
            print_displayid(DisplayId::from_extension(ext).unwrap());
        }
        _ => {}
    }
}

fn print_cta(cta: CTA<'_>) {
    println!("  Revision: {}", cta.revision());

    let cta_flags = cta.flags();
    if cta_flags.it_underscan {
        println!("  Underscans IT Video Formats by default");
    }
    if cta_flags.basic_audio {
        println!("  Basic audio support");
    }
    if cta_flags.ycc444 {
        println!("  Supports YCbCr 4:4:4");
    }
    if cta_flags.ycc422 {
        println!("  Supports YCbCr 4:2:2");
    }
    println!("  Native detailed modes: {}", cta_flags.native_dtds);

    for data_block in cta.data_blocks() {
        let tag = data_block.tag();
        println!("  {:?}:", tag);

        match tag {
            cta::DataBlockTag::Video => {
                for svd in data_block.svds() {
                    print_cta_svd(svd);
                }
            }
            cta::DataBlockTag::Ycbcr420 => {
                for svd in data_block.ycbcr420_svds() {
                    print_cta_svd(svd);
                }
            }
            cta::DataBlockTag::SpeakerAlloc => {
                let speaker_alloc = data_block.speaker_alloc().unwrap().speakers;

                if speaker_alloc.flw_frw {
                    println!("    FLw/FRw - Front Left/Right Wide");
                }
                if speaker_alloc.flc_frc {
                    println!("    FLc/FRc - Front Left/Right of Center");
                }
                if speaker_alloc.bc {
                    println!("    BC - Back Center");
                }
                if speaker_alloc.bl_br {
                    println!("    BL/BR - Back Left/Right");
                }
                if speaker_alloc.fc {
                    println!("    FC - Front Center");
                }
                if speaker_alloc.lfe1 {
                    println!("    LFE1 - Low Frequency Effects 1");
                }
                if speaker_alloc.fl_fr {
                    println!("    FL/FR - Front Left/Right");
                }
                if speaker_alloc.tpsil_tpsir {
                    println!("    TpSiL/TpSiR - Top Side Left/Right");
                }
                if speaker_alloc.sil_sir {
                    println!("    SiL/SiR - Side Left/Right");
                }
                if speaker_alloc.tpbc {
                    println!("    TpBC - Top Back Center");
                }
                if speaker_alloc.lfe2 {
                    println!("    LFE2 - Low Frequency Effects 2");
                }
                if speaker_alloc.ls_rs {
                    println!("    LS/RS - Left/Right Surround");
                }
                if speaker_alloc.tpfc {
                    println!("    TpFC - Top Front Center");
                }
                if speaker_alloc.tpc {
                    println!("    TpC - Top Center");
                }
                if speaker_alloc.tpfl_tpfr {
                    println!("    TpFL/TpFR - Top Front Left/Right");
                }
                if speaker_alloc.btfl_btfr {
                    println!("    BtFL/BtFR - Bottom Front Left/Right");
                }
                if speaker_alloc.btfc {
                    println!("    BtFC - Bottom Front Center");
                }
                if speaker_alloc.tpbl_tpbr {
                    println!("    TpBL/TpBR - Top Back Left/Right");
                }
            }
            cta::DataBlockTag::VideoCap => {
                let video_cap = data_block.video_cap().unwrap();
                println!(
                    "    YCbCr quantization: {}",
                    if video_cap.selectable_ycc_quantization_range {
                        "Selectable (via AVI YQ)"
                    } else {
                        "No Data"
                    }
                );
                println!(
                    "    RGB quantization: {}",
                    if video_cap.selectable_rgb_quantization_range {
                        "Selectable (via AVI Q)"
                    } else {
                        "No Data"
                    }
                );
                println!(
                    "    PT scan behavior: {}",
                    video_cap_over_underscan_name(video_cap.pt_over_underscan, "No Data")
                );
                println!(
                    "    IT scan behavior: {}",
                    video_cap_over_underscan_name(
                        video_cap.it_over_underscan,
                        "IT video formats not supported"
                    )
                );
                println!(
                    "    CE scan behavior: {}",
                    video_cap_over_underscan_name(
                        video_cap.ce_over_underscan,
                        "CE video formats not supported"
                    )
                );
            }
            cta::DataBlockTag::VesaDisplayDevice => {
                let vesa_dddb = data_block.vesa_dddb().unwrap();
                print_cta_vesa_dddb(vesa_dddb);
            }
            cta::DataBlockTag::Colorimetry => {
                let colorimetry = data_block.colorimetry().unwrap();
                if colorimetry.xvycc_601 {
                    println!("    xvYCC601");
                }
                if colorimetry.xvycc_709 {
                    println!("    xvYCC709");
                }
                if colorimetry.sycc_601 {
                    println!("    sYCC601")
                }
                if colorimetry.opycc_601 {
                    println!("    opYCC601");
                }
                if colorimetry.oprgb {
                    println!("    opRGB");
                }
                if colorimetry.bt2020_cycc {
                    println!("    BT2020cYCC");
                }
                if colorimetry.bt2020_ycc {
                    println!("    BT2020YCC");
                }
                if colorimetry.bt2020_rgb {
                    println!("    BT2020RGB");
                }
                if colorimetry.ictcp {
                    println!("    ICtCp");
                }
                if colorimetry.st2113_rgb {
                    println!("    ST2113RGB");
                }
            }
            cta::DataBlockTag::HdrStaticMetadata => {
                let hdr_static_metadata = data_block.hdr_static_metadata().unwrap();
                print_cta_hdr_static_metadata(hdr_static_metadata);
            }
            cta::DataBlockTag::HdrDynamicMetadata => {
                let hdr_dynamic_metadata = data_block.hdr_dynamic_metadata().unwrap();
                print_cta_hdr_dynamic_metadata(hdr_dynamic_metadata);
            }
            cta::DataBlockTag::VesaDisplayTransferCharacteristic => {
                let transfer_characteristics = data_block.vesa_transfer_characteristics().unwrap();
                print_cta_vesa_transfer_characteristics(transfer_characteristics);
            }
            cta::DataBlockTag::Audio => {
                for sad in data_block.sads() {
                    print_cta_sad(sad);
                }
            }
            cta::DataBlockTag::Ycbcr420CapMap => {
                let ycbcr420_cap_map = data_block.ycbcr420_cap_map().unwrap();
                print_ycbcr420_cap_map(&cta, ycbcr420_cap_map);
            }
            cta::DataBlockTag::Infoframe => {
                let infoframe = data_block.infoframe().unwrap();
                println!(
                    "    VSIFs: {}",
                    infoframe.inner().num_simultaneous_vsifs - 1
                );
                print_infoframes(infoframe.infoframes());
            }
            _ => {}
        }
    }

    let detailed_timing_defs = cta.detailed_timing_defs().collect::<Vec<_>>();
    if !detailed_timing_defs.is_empty() {
        println!("  Detailed Timing Descriptors:");
    }
    for (index, timing) in detailed_timing_defs.into_iter().enumerate() {
        print_detailed_timing_def(index, timing);
    }
}

fn print_displayid(displayid: DisplayId<'_>) {
    // const struct di_displayid_data_block *const *data_blocks;
    // const struct di_displayid_data_block *data_block;
    // enum di_displayid_data_block_tag tag;
    // size_t i;
    // const struct di_displayid_display_params *display_params;
    // const struct di_displayid_tiled_topo *tiled_topo;

    println!(
        "  Version: {}.{}",
        displayid.version(),
        displayid.revision()
    );

    // if is_displayid_base_block)
    // 	print!("  Display Product Type: %s\n",
    // 	       displayid_product_type_name(di_displayid_get_product_type(displayid)));
    // is_displayid_base_block = false;

    for data_block in displayid.data_blocks() {
        let tag = data_block.tag();
        println!("  {:?}:", tag);

        match tag {
            libdisplay_info::displayid::DataBlockTag::DisplayParams => {
                let display_params = data_block.display_params().unwrap();
                print_displayid_display_params(display_params);
            }
            libdisplay_info::displayid::DataBlockTag::TypeITiming => {
                print_displayid_type_i_timing_block(data_block);
            }
            libdisplay_info::displayid::DataBlockTag::TiledDisplayTopo => {
                let tiled_topo = data_block.tiled_topo().unwrap();
                print_displayid_tiled_topo(tiled_topo);
            }
            _ => {}
        }
    }
}

fn print_cta_svd(svd: cta::Svd) {
    print_vic(svd.vic);
    if svd.native {
        print!(" (native)");
    }
    println!();
}

fn print_vic(vic: u8) {
    print!("    VIC {:3}", vic);

    let Some(fmt) = cta::VideoFormat::from_vic(vic) else {
        return;
    };

    let mut v_active = fmt.v_active;
    if fmt.interlaced {
        v_active /= 2;
    }

    let h_blank = fmt.h_front + fmt.h_sync + fmt.h_back;
    let v_blank = fmt.v_front + fmt.v_sync + fmt.v_back;
    let h_total = (fmt.h_active + h_blank) as f64;

    let mut v_total = (v_active + v_blank) as f64;
    if fmt.interlaced {
        v_total += 0.5;
    }

    let refresh = fmt.pixel_clock_hz as f64 / (h_total * v_total);
    let h_freq_hz = fmt.pixel_clock_hz as f64 / h_total;
    let pixel_clock_mhz = fmt.pixel_clock_hz as f64 / (1000f64 * 1000f64);

    let buf = format!(
        "{}{}",
        fmt.v_active,
        if fmt.interlaced {
            "i"
        } else {
            Default::default()
        }
    );

    print!(":");
    print!(" {:5}x{:-5}", fmt.h_active, buf);
    print!(" {:10.6} Hz", refresh);
    print!(" {:?}", fmt.picture_aspect_ratio);
    print!(
        " {:8.3} kHz {:13.6} MHz",
        h_freq_hz / 1000f64,
        pixel_clock_mhz
    );
}

fn video_cap_over_underscan_name(
    over_underscan: cta::VideoCapOverUnderscan,
    unknown: &'static str,
) -> &'static str {
    match over_underscan {
        cta::VideoCapOverUnderscan::UnknownOverUnderscan => unknown,
        cta::VideoCapOverUnderscan::AlwaysOverscan => "Always Overscanned",
        cta::VideoCapOverUnderscan::AlwaysUnderscan => "Always Underscanned",
        cta::VideoCapOverUnderscan::BothOverUnderscan => "Supports both over- and underscan",
    }
}

fn print_cta_hdr_static_metadata(metadata: cta::HdrStaticMetadataBlock) {
    println!("    Electro optical transfer functions:");
    if let Some(eotfs) = metadata.eotfs {
        if eotfs.traditional_sdr {
            println!("      Traditional gamma - SDR luminance range");
        }
        if eotfs.traditional_hdr {
            println!("      Traditional gamma - HDR luminance range");
        }
        if eotfs.pq {
            println!("      SMPTE ST2084");
        }
        if eotfs.hlg {
            println!("      Hybrid Log-Gamma");
        }
    }

    if let Some(descriptors) = metadata.descriptors {
        println!("    Supported static metadata descriptors:");
        if descriptors.type1 {
            println!("      Static metadata type 1");
        }
    }

    if let Some(desired_content_max_luminance) = metadata.desired_content_max_luminance {
        println!(
            "    Desired content max luminance: {} ({:.3} cd/m^2)",
            encode_max_luminance(desired_content_max_luminance),
            desired_content_max_luminance
        )
    }
    if let Some(desired_content_max_frame_avg_luminance) =
        metadata.desired_content_max_frame_avg_luminance
    {
        println!(
            "    Desired content max frame-average luminance: {} ({:.3} cd/m^2)",
            encode_max_luminance(desired_content_max_frame_avg_luminance),
            desired_content_max_frame_avg_luminance
        )
    }
    if let Some(desired_content_min_luminance) = metadata.desired_content_min_luminance {
        println!(
            "    Desired content min luminance: {} ({:.3} cd/m^2)",
            encode_min_luminance(
                desired_content_min_luminance,
                metadata.desired_content_max_luminance.unwrap_or_default()
            ),
            desired_content_min_luminance
        )
    }
}

fn print_cta_hdr_dynamic_metadata(metadata: cta::HdrDynamicMetadataBlock) {
    if let Some(type1) = metadata.type1 {
        println!("    HDR Dynamic Metadata Type 1");
        println!("      Version: {}", type1.type_1_hdr_metadata_version);
    }

    if let Some(type2) = metadata.type2 {
        println!("    HDR Dynamic Metadata Type 2");
        println!("      Version: {}", type2.ts_103_433_spec_version);
        if type2.ts_103_433_1_capable {
            println!("      ETSI TS 103 433-1 capable");
        }
        if type2.ts_103_433_2_capable {
            println!("      ETSI TS 103 433-2 [i.12] capable");
        }
        if type2.ts_103_433_3_capable {
            println!("      ETSI TS 103 433-3 [i.13] capable");
        }
    }
    if let Some(_type3) = metadata.type3 {
        println!("    HDR Dynamic Metadata Type 3");
    }
    if let Some(type4) = metadata.type4 {
        println!("    HDR Dynamic Metadata Type 4");
        println!("      Version: {}", type4.type_4_hdr_metadata_version);
    }
    if let Some(type256) = metadata.type256 {
        println!("    HDR Dynamic Metadata Type 256");
        println!("      Version: {}", type256.graphics_overlay_flag_version);
    }
}

fn encode_max_luminance(max: f32) -> u8 {
    if max == 0f32 {
        0
    } else {
        (f32::log2(max / 50f32) * 32f32) as u8
    }
}

fn encode_min_luminance(min: f32, max: f32) -> u8 {
    if min == 0f32 {
        0
    } else {
        (255f32 * f32::sqrt(min / max * 100f32)) as u8
    }
}

fn print_cta_sad(sad: cta::Sad) {
    println!("    {:?}:", sad.format);
    if let Some(max_channels) = sad.max_channels {
        println!("      Max channels: {}", max_channels);
    }

    if let Some(mpegh_3d) = sad.mpegh_3d {
        println!("      MPEG-H 3D Audio Level: {:?}", mpegh_3d.level);
    }

    if let Some(supported_sample_rates) = sad.supported_sample_rates {
        print!("      Supported sample rates (kHz):");
        if supported_sample_rates.has_192_khz {
            print!(" 192");
        }
        if supported_sample_rates.has_176_4_khz {
            print!(" 176.4");
        }
        if supported_sample_rates.has_96_khz {
            print!(" 96");
        }
        if supported_sample_rates.has_88_2_khz {
            print!(" 88.2");
        }
        if supported_sample_rates.has_48_khz {
            print!(" 48");
        }
        if supported_sample_rates.has_44_1_khz {
            print!(" 44.1");
        }
        if supported_sample_rates.has_32_khz {
            print!(" 32");
        }
        println!();
    }

    if let Some(lpcm) = sad.lpcm {
        print!("      Supported sample sizes (bits):");
        if lpcm.has_sample_size_24_bits {
            print!(" 24");
        }
        if lpcm.has_sample_size_20_bits {
            print!(" 20");
        }
        if lpcm.has_sample_size_16_bits {
            print!(" 16");
        }
        println!();
    }

    if let Some(max_bitrate_kbs) = sad.max_bitrate_kbs {
        println!("      Maximum bit rate: {} kb/s", max_bitrate_kbs);
    }

    if let Some(enhanced_ac3) = sad.enhanced_ac3 {
        if enhanced_ac3.supports_joint_object_coding {
            println!("      Supports Joint Object Coding");
        }
        if enhanced_ac3.supports_joint_object_coding_ACMOD28 {
            println!("      Supports Joint Object Coding with ACMOD28");
        }
    }

    if let Some(mat) = sad.mat {
        if mat.supports_object_audio_and_channel_based {
            println!("      Supports Dolby TrueHD, object audio PCM and channel-based PCM");
            println!(
                "      Hash calculation {}required for object audio PCM or channel-based PCM",
                if mat.requires_hash_calculation {
                    ""
                } else {
                    "not "
                }
            );
        } else {
            println!("      Supports only Dolby TrueHD");
        }
    }

    if let Some(wma_pro) = sad.wma_pro {
        println!("      Profile: {}", wma_pro.profile);
    }

    if let Some(mpegh_3d) = sad.mpegh_3d {
        if mpegh_3d.low_complexity_profile {
            println!("      Supports MPEG-H 3D Audio Low Complexity Profile");
        }
        if mpegh_3d.baseline_profile {
            println!("      Supports MPEG-H 3D Audio Baseline Profile");
        }
    }

    if let Some(mpeg_aac) = sad.mpeg_aac {
        println!(
            "      AAC audio frame lengths:{}%{}",
            if mpeg_aac.has_frame_length_1024 {
                " 1024_TL"
            } else {
                Default::default()
            },
            if mpeg_aac.has_frame_length_960 {
                " 960_TL"
            } else {
                Default::default()
            }
        );
    }

    if let Some(mpeg_surround) = sad.mpeg_surround {
        println!(
            "      Supports {} signaled MPEG Surround data",
            if mpeg_surround.signaling == cta::SadMpegSurroundSignaling::Implicit {
                "only implicitly"
            } else {
                "implicitly and explicitly"
            }
        );
    }

    if let Some(mpeg_aac_le) = sad.mpeg_aac_le {
        if mpeg_aac_le.supports_multichannel_sound {
            println!("      Supports 22.2ch System H");
        }
    }
}

fn print_cta_vesa_dddb(dddb: cta::VesaDddb) {
    print!("    Interface Type: {:?}", dddb.interface_type);
    if let Some(num_channels) = dddb.num_channels {
        let kind = match dddb.interface_type {
            cta::VesaDddbInterfaceType::LVDS | cta::VesaDddbInterfaceType::RSDS => "lanes",
            _ => "channels",
        };
        print!(" {} {}", num_channels, kind);
    }
    println!();

    println!(
        "    Interface Standard Version: {}.{}",
        dddb.interface_version, dddb.interface_release
    );

    println!(
        "    Content Protection Support: {:?}",
        dddb.content_protection
    );

    println!(
        "    Minimum Clock Frequency: {} MHz",
        dddb.min_clock_freq_mhz
    );
    println!(
        "    Maximum Clock Frequency: {} MHz",
        dddb.max_clock_freq_mhz
    );
    println!(
        "    Device Native Pixel Format: {}x{}",
        dddb.native_horiz_pixels, dddb.native_vert_pixels
    );
    println!("    Aspect Ratio: {:.2}", dddb.aspect_ratio);
    println!("    Default Orientation: {:?}", dddb.default_orientation);
    println!("    Rotation Capability: {:?}", dddb.rotation_cap);
    println!("    Zero Pixel Location: {:?}", dddb.zero_pixel_location);
    println!("    Scan Direction: {:?}", dddb.scan_direction);
    println!("    Subpixel Information: {:?}", dddb.subpixel_layout);
    println!(
        "    Horizontal and vertical dot/pixel pitch: {:.2} x {:.2} mm",
        dddb.horiz_pitch_mm, dddb.vert_pitch_mm
    );
    println!("    Dithering: {:?}", dddb.dithering_type);
    println!(
        "    Direct Drive: {}",
        if dddb.direct_drive { "Yes" } else { "No" }
    );
    println!(
        "    Overdrive {}recommended",
        if dddb.overdrive_not_recommended {
            "not"
        } else {
            Default::default()
        }
    );
    println!(
        "    Deinterlacing: {}",
        if dddb.deinterlacing { "Yes" } else { "No" }
    );

    println!(
        "    Audio Support: {}",
        if dddb.audio_support { "Yes" } else { "No" }
    );
    println!(
        "    Separate Audio Inputs Provided: {}",
        if dddb.separate_audio_inputs {
            "Yes"
        } else {
            "No"
        }
    );
    println!(
        "    Audio Input Override: {}",
        if dddb.audio_input_override {
            "Yes"
        } else {
            "No"
        }
    );
    if dddb.audio_delay_provided {
        println!("    Audio Delay: {} ms", dddb.audio_delay_ms);
    } else {
        println!("    Audio Delay: no information provided");
    }

    println!(
        "    Frame Rate/Mode Conversion: {:?}",
        dddb.frame_rate_conversion
    );
    if let Some(frame_rate_range_hz) = dddb.frame_rate_range_hz {
        println!(
            "    Frame Rate Range: {} fps +/- {} fps",
            dddb.frame_rate_native_hz, frame_rate_range_hz
        );
    } else {
        println!("    Nominal Frame Rate: {} fps", dddb.frame_rate_native_hz);
        println!(
            "    Color Bit Depth: {} @ interface, {} @ display",
            dddb.bit_depth_interface, dddb.bit_depth_display
        );
    }

    if dddb.additional_primary_chromaticities_len > 0 {
        println!("    Additional Primary Chromaticities:");
        for (i, additional_primary_chromaticities) in dddb
            .additional_primary_chromaticities
            .iter()
            .take(dddb.additional_primary_chromaticities_len)
            .enumerate()
        {
            println!(
                "      Primary {}:   {:.4}, {:.4}",
                4 + i,
                additional_primary_chromaticities.x,
                additional_primary_chromaticities.y
            );
        }
    }

    println!(
        "    Response Time {:?}: {} ms",
        dddb.resp_time_transition, dddb.resp_time_ms
    );
    println!(
        "    Overscan: {}% x {}%",
        dddb.overscan_horiz_pct, dddb.overscan_vert_pct
    );
}

fn print_cta_vesa_transfer_characteristics(tf: cta::VesaTransferCharacteristics) {
    print!("    {:?}", tf.usage);

    print!(" transfer characteristics:");
    for point in tf.points.iter().take(tf.points_len as usize) {
        print!(" {}", f32::round(point * 1023f32) as u16);
    }
    println!();
}

fn print_ycbcr420_cap_map(cta: &cta::CTA<'_>, map: cta::Ycbcr420CapMapRef) {
    for data_block in cta.data_blocks() {
        if data_block.tag() != cta::DataBlockTag::Video {
            continue;
        }

        for (index, svd) in data_block.svds().enumerate() {
            if map.di_cta_ycbcr420_cap_map_supported(index) {
                print_cta_svd(svd)
            }
        }
    }
}

fn print_infoframes(info_frames: impl Iterator<Item = cta::InfoframeDescriptor>) {
    for infoframe in info_frames {
        println!("    {:?}", infoframe.type_);
    }
}

fn print_displayid_display_params(params: displayid::DisplayParams) {
    println!(
        "    Image size: {:.1} mm x {:.1} mm",
        params.horiz_image_mm, params.vert_image_mm
    );
    println!(
        "    Display native pixel format: {}x{}",
        params.horiz_pixels, params.vert_pixels
    );

    if let Some(features) = params.features {
        println!("    Feature support flags:");
        if features.audio {
            println!("      Audio support on video interface");
        }
        if features.separate_audio_inputs {
            println!("      Separate audio inputs provided");
        }
        if features.audio_input_override {
            println!("      Audio input override");
        }
        if features.power_management {
            println!("      Power management (DPM)");
        }
        if features.fixed_timing {
            println!("      Fixed timing");
        }
        if features.fixed_pixel_format {
            println!("      Fixed pixel format");
        }
        if features.ai {
            println!("      Support ACP, ISRC1, or ISRC2packets");
        }
        if features.deinterlacing {
            println!("      De-interlacing");
        }
    }

    if let Some(gamma) = params.gamma {
        println!("    Gamma: {:.2}", gamma);
    }
    println!("    Aspect ratio: {:.2}", params.aspect_ratio);
    println!("    Dynamic bpc native: {}", params.bits_per_color_native);
    println!("    Dynamic bpc overall: {}", params.bits_per_color_overall);
}

fn print_displayid_type_i_timing_block(data_block: &displayid::DataBlockRef) {
    for timing in data_block.type_i_timings() {
        print_displayid_type_i_timing(timing);
    }
}

fn print_displayid_type_i_timing(t: displayid::TypeIIIVIITiming) {
    let (horiz_ratio, vert_ratio) = displayid_type_i_timing_aspect_ratio(t.aspect_ratio);

    let horiz_total = t.horiz_active + t.horiz_blank;
    let vert_total = t.vert_active + t.vert_blank;
    let pixel_clock_hz = t.pixel_clock_mhz * 1000f64 * 1000f64;
    let refresh = pixel_clock_hz / (horiz_total * vert_total) as f64;
    let horiz_freq_hz = pixel_clock_hz / horiz_total as f64;

    print!("    DTD:");
    print!(" {:5}x{:-5}", t.horiz_active, t.vert_active);
    if t.interlaced {
        print!("i");
    }
    print!(" {:10.6} Hz", refresh);
    print!(" {:3}:{:-3}", horiz_ratio, vert_ratio);
    print!(
        " {:8.3} kHz {:13.6} MHz",
        horiz_freq_hz / 1000f64,
        t.pixel_clock_mhz
    );
    print!(" (aspect ");
    if t.aspect_ratio == displayid::TimingAspectRatio::Undefined {
        print!("undefined");
    } else {
        print!("{}:{}", horiz_ratio, vert_ratio);
    }
    print!(", {:?}", t.stereo_3d);
    if t.preferred {
        print!(", preferred");
    }
    println!(")");

    let horiz_back_porch = t.horiz_blank - t.horiz_sync_width - t.horiz_offset;
    print!(
        "               Hfront {:4} Hsync {:3} Hback {:4} Hpol {:?}",
        t.horiz_offset, t.horiz_sync_width, horiz_back_porch, t.horiz_sync_polarity
    );
    println!();

    let vert_back_porch = t.vert_blank - t.vert_sync_width - t.vert_offset;
    print!(
        "               Vfront {:4} Vsync {:3} Vback {:4} Vpol {:?}",
        t.vert_offset, t.vert_sync_width, vert_back_porch, t.vert_sync_polarity
    );
    println!();
}

fn displayid_type_i_timing_aspect_ratio(ratio: displayid::TimingAspectRatio) -> (i32, i32) {
    match ratio {
        displayid::TimingAspectRatio::_1_1 => (1, 1),
        displayid::TimingAspectRatio::_5_4 => (5, 4),
        displayid::TimingAspectRatio::_4_3 => (4, 3),
        displayid::TimingAspectRatio::_15_9 => (15, 9),
        displayid::TimingAspectRatio::_16_9 => (16, 9),
        displayid::TimingAspectRatio::_16_10 => (16, 10),
        displayid::TimingAspectRatio::_64_27 => (64, 27),
        displayid::TimingAspectRatio::_256_135 => (256, 135),
        displayid::TimingAspectRatio::Undefined => (0, 0),
    }
}

fn print_displayid_tiled_topo(tiled_topo: displayid::TiledTopo) {
    if let Some(caps) = tiled_topo.caps {
        println!("    Capabilities:");
        println!(
            "      Behavior if it is the only tile: {:?}",
            caps.single_recv_behavior
        );
        println!(
            "      Behavior if more than one tile and fewer than total number of tiles: {:?}",
            caps.missing_recv_behavior
        );

        if caps.single_enclosure {
            println!("    Tiled display consists of a single physical display enclosure");
        } else {
            println!("    Tiled display consists of multiple physical display enclosures");
        }
    }

    println!(
        "    Num horizontal tiles: {} Num vertical tiles: {}",
        tiled_topo.total_horiz_tiles, tiled_topo.total_vert_tiles
    );
    println!(
        "    Tile location: {}, {}",
        tiled_topo.horiz_tile_location - 1,
        tiled_topo.vert_tile_location - 1
    );
    println!(
        "    Tile resolution: {}x{}",
        tiled_topo.horiz_tile_pixels, tiled_topo.vert_tile_lines
    );

    if let Some(bezel) = tiled_topo.bezel {
        println!("    Top bevel size: {:.1} pixels", bezel.top_px);
        println!("    Bottom bevel size: {:.1} pixels", bezel.bottom_px);
        println!("    Right bevel size: {:.1} pixels", bezel.right_px);
        println!("    Left bevel size: {:.1} pixels", bezel.left_px);
    }

    println!(
        "    Tiled Display Manufacturer/Vendor ID: {:?}",
        tiled_topo.vendor_id
    );
    println!(
        "    Tiled Display Product ID Code: {}",
        tiled_topo.product_code
    );
    println!(
        "    Tiled Display Serial Number: {}",
        tiled_topo.serial_number
    );
}

fn has_established_timings(timings: &EstablishedTimings) -> bool {
    timings.has_720x400_70hz
        || timings.has_720x400_88hz
        || timings.has_640x480_60hz
        || timings.has_640x480_67hz
        || timings.has_640x480_72hz
        || timings.has_640x480_75hz
        || timings.has_800x600_56hz
        || timings.has_800x600_60hz
        || timings.has_800x600_72hz
        || timings.has_800x600_75hz
        || timings.has_832x624_75hz
        || timings.has_1024x768_87hz_interlaced
        || timings.has_1024x768_60hz
        || timings.has_1024x768_70hz
        || timings.has_1024x768_75hz
        || timings.has_1280x1024_75hz
        || timings.has_1152x870_75hz
}
