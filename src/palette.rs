use crate::Color;

/// A color palette with 11 shades from lightest (c50) to darkest (c950).
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub c50: Color,
    pub c100: Color,
    pub c200: Color,
    pub c300: Color,
    pub c400: Color,
    pub c500: Color,
    pub c600: Color,
    pub c700: Color,
    pub c800: Color,
    pub c900: Color,
    pub c950: Color,
}

pub mod tailwind {
    use super::Palette;
    use crate::Color;

    pub const SLATE: Palette = Palette {
        c50: Color::Rgb(248, 250, 252),
        c100: Color::Rgb(241, 245, 249),
        c200: Color::Rgb(226, 232, 240),
        c300: Color::Rgb(203, 213, 225),
        c400: Color::Rgb(148, 163, 184),
        c500: Color::Rgb(100, 116, 139),
        c600: Color::Rgb(71, 85, 105),
        c700: Color::Rgb(51, 65, 85),
        c800: Color::Rgb(30, 41, 59),
        c900: Color::Rgb(15, 23, 42),
        c950: Color::Rgb(2, 6, 23),
    };

    pub const GRAY: Palette = Palette {
        c50: Color::Rgb(249, 250, 251),
        c100: Color::Rgb(243, 244, 246),
        c200: Color::Rgb(229, 231, 235),
        c300: Color::Rgb(209, 213, 219),
        c400: Color::Rgb(156, 163, 175),
        c500: Color::Rgb(107, 114, 128),
        c600: Color::Rgb(75, 85, 99),
        c700: Color::Rgb(55, 65, 81),
        c800: Color::Rgb(31, 41, 55),
        c900: Color::Rgb(17, 24, 39),
        c950: Color::Rgb(3, 7, 18),
    };

    pub const ZINC: Palette = Palette {
        c50: Color::Rgb(250, 250, 250),
        c100: Color::Rgb(244, 244, 245),
        c200: Color::Rgb(228, 228, 231),
        c300: Color::Rgb(212, 212, 216),
        c400: Color::Rgb(161, 161, 170),
        c500: Color::Rgb(113, 113, 122),
        c600: Color::Rgb(82, 82, 91),
        c700: Color::Rgb(63, 63, 70),
        c800: Color::Rgb(39, 39, 42),
        c900: Color::Rgb(24, 24, 27),
        c950: Color::Rgb(9, 9, 11),
    };

    pub const NEUTRAL: Palette = Palette {
        c50: Color::Rgb(250, 250, 250),
        c100: Color::Rgb(245, 245, 245),
        c200: Color::Rgb(229, 229, 229),
        c300: Color::Rgb(212, 212, 212),
        c400: Color::Rgb(163, 163, 163),
        c500: Color::Rgb(115, 115, 115),
        c600: Color::Rgb(82, 82, 82),
        c700: Color::Rgb(64, 64, 64),
        c800: Color::Rgb(38, 38, 38),
        c900: Color::Rgb(23, 23, 23),
        c950: Color::Rgb(10, 10, 10),
    };

    pub const STONE: Palette = Palette {
        c50: Color::Rgb(250, 250, 249),
        c100: Color::Rgb(245, 245, 244),
        c200: Color::Rgb(231, 229, 228),
        c300: Color::Rgb(214, 211, 209),
        c400: Color::Rgb(168, 162, 158),
        c500: Color::Rgb(120, 113, 108),
        c600: Color::Rgb(87, 83, 78),
        c700: Color::Rgb(68, 64, 60),
        c800: Color::Rgb(41, 37, 36),
        c900: Color::Rgb(28, 25, 23),
        c950: Color::Rgb(12, 10, 9),
    };

    pub const RED: Palette = Palette {
        c50: Color::Rgb(254, 242, 242),
        c100: Color::Rgb(254, 226, 226),
        c200: Color::Rgb(254, 202, 202),
        c300: Color::Rgb(252, 165, 165),
        c400: Color::Rgb(248, 113, 113),
        c500: Color::Rgb(239, 68, 68),
        c600: Color::Rgb(220, 38, 38),
        c700: Color::Rgb(185, 28, 28),
        c800: Color::Rgb(153, 27, 27),
        c900: Color::Rgb(127, 29, 29),
        c950: Color::Rgb(69, 10, 10),
    };

    pub const ORANGE: Palette = Palette {
        c50: Color::Rgb(255, 247, 237),
        c100: Color::Rgb(255, 237, 213),
        c200: Color::Rgb(254, 215, 170),
        c300: Color::Rgb(253, 186, 116),
        c400: Color::Rgb(251, 146, 60),
        c500: Color::Rgb(249, 115, 22),
        c600: Color::Rgb(234, 88, 12),
        c700: Color::Rgb(194, 65, 12),
        c800: Color::Rgb(154, 52, 18),
        c900: Color::Rgb(124, 45, 18),
        c950: Color::Rgb(67, 20, 7),
    };

    pub const AMBER: Palette = Palette {
        c50: Color::Rgb(255, 251, 235),
        c100: Color::Rgb(254, 243, 199),
        c200: Color::Rgb(253, 230, 138),
        c300: Color::Rgb(252, 211, 77),
        c400: Color::Rgb(251, 191, 36),
        c500: Color::Rgb(245, 158, 11),
        c600: Color::Rgb(217, 119, 6),
        c700: Color::Rgb(180, 83, 9),
        c800: Color::Rgb(146, 64, 14),
        c900: Color::Rgb(120, 53, 15),
        c950: Color::Rgb(69, 26, 3),
    };

    pub const YELLOW: Palette = Palette {
        c50: Color::Rgb(254, 252, 232),
        c100: Color::Rgb(254, 249, 195),
        c200: Color::Rgb(254, 240, 138),
        c300: Color::Rgb(253, 224, 71),
        c400: Color::Rgb(250, 204, 21),
        c500: Color::Rgb(234, 179, 8),
        c600: Color::Rgb(202, 138, 4),
        c700: Color::Rgb(161, 98, 7),
        c800: Color::Rgb(133, 77, 14),
        c900: Color::Rgb(113, 63, 18),
        c950: Color::Rgb(66, 32, 6),
    };

    pub const LIME: Palette = Palette {
        c50: Color::Rgb(247, 254, 231),
        c100: Color::Rgb(236, 252, 203),
        c200: Color::Rgb(217, 249, 157),
        c300: Color::Rgb(190, 242, 100),
        c400: Color::Rgb(163, 230, 53),
        c500: Color::Rgb(132, 204, 22),
        c600: Color::Rgb(101, 163, 13),
        c700: Color::Rgb(77, 124, 15),
        c800: Color::Rgb(63, 98, 18),
        c900: Color::Rgb(54, 83, 20),
        c950: Color::Rgb(26, 46, 5),
    };

    pub const GREEN: Palette = Palette {
        c50: Color::Rgb(240, 253, 244),
        c100: Color::Rgb(220, 252, 231),
        c200: Color::Rgb(187, 247, 208),
        c300: Color::Rgb(134, 239, 172),
        c400: Color::Rgb(74, 222, 128),
        c500: Color::Rgb(34, 197, 94),
        c600: Color::Rgb(22, 163, 74),
        c700: Color::Rgb(21, 128, 61),
        c800: Color::Rgb(22, 101, 52),
        c900: Color::Rgb(20, 83, 45),
        c950: Color::Rgb(5, 46, 22),
    };

    pub const EMERALD: Palette = Palette {
        c50: Color::Rgb(236, 253, 245),
        c100: Color::Rgb(209, 250, 229),
        c200: Color::Rgb(167, 243, 208),
        c300: Color::Rgb(110, 231, 183),
        c400: Color::Rgb(52, 211, 153),
        c500: Color::Rgb(16, 185, 129),
        c600: Color::Rgb(5, 150, 105),
        c700: Color::Rgb(4, 120, 87),
        c800: Color::Rgb(6, 95, 70),
        c900: Color::Rgb(6, 78, 59),
        c950: Color::Rgb(2, 44, 34),
    };

    pub const TEAL: Palette = Palette {
        c50: Color::Rgb(240, 253, 250),
        c100: Color::Rgb(204, 251, 241),
        c200: Color::Rgb(153, 246, 228),
        c300: Color::Rgb(94, 234, 212),
        c400: Color::Rgb(45, 212, 191),
        c500: Color::Rgb(20, 184, 166),
        c600: Color::Rgb(13, 148, 136),
        c700: Color::Rgb(15, 118, 110),
        c800: Color::Rgb(17, 94, 89),
        c900: Color::Rgb(19, 78, 74),
        c950: Color::Rgb(4, 47, 46),
    };

    pub const CYAN: Palette = Palette {
        c50: Color::Rgb(236, 254, 255),
        c100: Color::Rgb(207, 250, 254),
        c200: Color::Rgb(165, 243, 252),
        c300: Color::Rgb(103, 232, 249),
        c400: Color::Rgb(34, 211, 238),
        c500: Color::Rgb(6, 182, 212),
        c600: Color::Rgb(8, 145, 178),
        c700: Color::Rgb(14, 116, 144),
        c800: Color::Rgb(21, 94, 117),
        c900: Color::Rgb(22, 78, 99),
        c950: Color::Rgb(8, 51, 68),
    };

    pub const SKY: Palette = Palette {
        c50: Color::Rgb(240, 249, 255),
        c100: Color::Rgb(224, 242, 254),
        c200: Color::Rgb(186, 230, 253),
        c300: Color::Rgb(125, 211, 252),
        c400: Color::Rgb(56, 189, 248),
        c500: Color::Rgb(14, 165, 233),
        c600: Color::Rgb(2, 132, 199),
        c700: Color::Rgb(3, 105, 161),
        c800: Color::Rgb(7, 89, 133),
        c900: Color::Rgb(12, 74, 110),
        c950: Color::Rgb(8, 47, 73),
    };

    pub const BLUE: Palette = Palette {
        c50: Color::Rgb(239, 246, 255),
        c100: Color::Rgb(219, 234, 254),
        c200: Color::Rgb(191, 219, 254),
        c300: Color::Rgb(147, 197, 253),
        c400: Color::Rgb(96, 165, 250),
        c500: Color::Rgb(59, 130, 246),
        c600: Color::Rgb(37, 99, 235),
        c700: Color::Rgb(29, 78, 216),
        c800: Color::Rgb(30, 64, 175),
        c900: Color::Rgb(30, 58, 138),
        c950: Color::Rgb(23, 37, 84),
    };

    pub const INDIGO: Palette = Palette {
        c50: Color::Rgb(238, 242, 255),
        c100: Color::Rgb(224, 231, 255),
        c200: Color::Rgb(199, 210, 254),
        c300: Color::Rgb(165, 180, 252),
        c400: Color::Rgb(129, 140, 248),
        c500: Color::Rgb(99, 102, 241),
        c600: Color::Rgb(79, 70, 229),
        c700: Color::Rgb(67, 56, 202),
        c800: Color::Rgb(55, 48, 163),
        c900: Color::Rgb(49, 46, 129),
        c950: Color::Rgb(30, 27, 75),
    };

    pub const VIOLET: Palette = Palette {
        c50: Color::Rgb(245, 243, 255),
        c100: Color::Rgb(237, 233, 254),
        c200: Color::Rgb(221, 214, 254),
        c300: Color::Rgb(196, 181, 253),
        c400: Color::Rgb(167, 139, 250),
        c500: Color::Rgb(139, 92, 246),
        c600: Color::Rgb(124, 58, 237),
        c700: Color::Rgb(109, 40, 217),
        c800: Color::Rgb(91, 33, 182),
        c900: Color::Rgb(76, 29, 149),
        c950: Color::Rgb(46, 16, 101),
    };

    pub const PURPLE: Palette = Palette {
        c50: Color::Rgb(250, 245, 255),
        c100: Color::Rgb(243, 232, 255),
        c200: Color::Rgb(233, 213, 255),
        c300: Color::Rgb(216, 180, 254),
        c400: Color::Rgb(192, 132, 252),
        c500: Color::Rgb(168, 85, 247),
        c600: Color::Rgb(147, 51, 234),
        c700: Color::Rgb(126, 34, 206),
        c800: Color::Rgb(107, 33, 168),
        c900: Color::Rgb(88, 28, 135),
        c950: Color::Rgb(59, 7, 100),
    };

    pub const FUCHSIA: Palette = Palette {
        c50: Color::Rgb(253, 244, 255),
        c100: Color::Rgb(250, 232, 255),
        c200: Color::Rgb(245, 208, 254),
        c300: Color::Rgb(240, 171, 252),
        c400: Color::Rgb(232, 121, 249),
        c500: Color::Rgb(217, 70, 239),
        c600: Color::Rgb(192, 38, 211),
        c700: Color::Rgb(162, 28, 175),
        c800: Color::Rgb(134, 25, 143),
        c900: Color::Rgb(112, 26, 117),
        c950: Color::Rgb(74, 4, 78),
    };

    pub const PINK: Palette = Palette {
        c50: Color::Rgb(253, 242, 248),
        c100: Color::Rgb(252, 231, 243),
        c200: Color::Rgb(251, 207, 232),
        c300: Color::Rgb(249, 168, 212),
        c400: Color::Rgb(244, 114, 182),
        c500: Color::Rgb(236, 72, 153),
        c600: Color::Rgb(219, 39, 119),
        c700: Color::Rgb(190, 24, 93),
        c800: Color::Rgb(157, 23, 77),
        c900: Color::Rgb(131, 24, 67),
        c950: Color::Rgb(80, 7, 36),
    };

    pub const ROSE: Palette = Palette {
        c50: Color::Rgb(255, 241, 242),
        c100: Color::Rgb(255, 228, 230),
        c200: Color::Rgb(254, 205, 211),
        c300: Color::Rgb(253, 164, 175),
        c400: Color::Rgb(251, 113, 133),
        c500: Color::Rgb(244, 63, 94),
        c600: Color::Rgb(225, 29, 72),
        c700: Color::Rgb(190, 18, 60),
        c800: Color::Rgb(159, 18, 57),
        c900: Color::Rgb(136, 19, 55),
        c950: Color::Rgb(76, 5, 25),
    };
}
