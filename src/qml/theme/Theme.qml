pragma Singleton
import QtQuick

QtObject {
    id: root

    property color base: theme.base
    property color mantle: theme.mantle
    property color crust: theme.crust
    property color surface: theme.surface
    property color overlay: theme.overlay
    property color text: theme.text
    property color subtext: theme.subtext
    property color muted: theme.muted
    property color accent: theme.accent
    property color success: theme.success
    property color warning: theme.warning
    property color error: theme.error

    property int radiusSmall: config.radiusSmall
    property int radiusMedium: config.radiusMedium
    property int radiusLarge: config.radiusLarge
    readonly property real baseFontSize: {
        var pointSize = Qt.application.font.pointSize
        return pointSize > 0 ? pointSize : 10
    }
    readonly property real uiScale: Math.max(1.0, baseFontSize / 10.0)
    readonly property int spacing: Math.round(8 * uiScale)
    readonly property int fontSmall: Math.max(9, Math.round(baseFontSize - 1))
    readonly property int fontNormal: Math.max(10, Math.round(baseFontSize))
    readonly property int fontLarge: Math.max(12, Math.round(baseFontSize + 2))
    readonly property int controlSize: Math.round(32 * uiScale)
    readonly property int compactControlSize: Math.round(28 * uiScale)
    readonly property int titleBarHeight: Math.round(34 * uiScale)
    readonly property int toolbarRowHeight: Math.round(44 * uiScale)
    property bool transparencyEnabled: config.transparencyEnabled
    property real transparencyLevel: Math.max(0, Math.min(1, config.transparencyLevel))
    property bool animationsEnabled: config.animationsEnabled

    readonly property int animDurationFast: animationsEnabled ? config.animDurationFast : 0
    readonly property int animDuration: animationsEnabled ? config.animDuration : 0
    readonly property int animDurationSlow: animationsEnabled ? config.animDurationSlow : 0
    function _curveToEasing(name) {
        switch (name) {
        case "Linear":       return Easing.Linear
        case "InCubic":      return Easing.InCubic
        case "OutCubic":     return Easing.OutCubic
        case "InOutCubic":   return Easing.InOutCubic
        case "OutBack":      return Easing.OutBack
        case "InOutQuad":    return Easing.InOutQuad
        case "OutQuad":      return Easing.OutQuad
        case "OutExpo":      return Easing.OutExpo
        case "InOutExpo":    return Easing.InOutExpo
        case "Bezier":       return Easing.BezierSpline
        default:             return Easing.OutCubic
        }
    }

    readonly property int animEasingEnter: _curveToEasing(config.animCurveEnter)
    readonly property int animEasingExit: _curveToEasing(config.animCurveExit)
    readonly property int animEasingTransition: _curveToEasing(config.animCurveTransition)
    // Material-design standard easing curve. Used when any of the above
    // resolves to Easing.BezierSpline — every Behavior that consumes one
    // of the animEasing* properties also sets easing.bezierCurve to this.
    readonly property var animBezierCurve: [0.4, 0.0, 0.2, 1.0, 1.0, 1.0]

    // ── Glass Effects ──────────────────────────────────────────────────
    // Sourced from the [effects] section in the active theme's .toml.
    // When absent, all effects default to "off" (hasEffects = false).
    property bool hasEffects: theme.hasEffects
    property real sidebarOpacity: theme.sidebarOpacity
    property real contentOpacity: theme.contentOpacity
    property real toolbarOpacity: theme.toolbarOpacity
    property bool gradientEnabled: theme.gradientEnabled
    property color gradientColor: theme.gradientColor
    property string gradientDirection: theme.gradientDirection
    property bool glowEnabled: theme.glowEnabled
    property color glowColor: theme.glowColor
    property real glowRadius: theme.glowRadius
    property real glowOpacity: theme.glowOpacity
    property bool blurEnabled: theme.blurEnabled
    property real blurRadius: theme.blurRadius
    property bool noiseEnabled: theme.noiseEnabled
    property real noiseOpacity: theme.noiseOpacity
    property real saturation: theme.saturation
    property bool refractionEnabled: theme.refractionEnabled
    property real refractionStrength: theme.refractionStrength

    // Whether the compositor provides blur behind the window
    property bool compositorBlurAvailable: theme.compositorBlurAvailable

    // Effective blur radius: reduced when compositor provides blur
    readonly property real effectiveBlurRadius: {
        if (!hasEffects || !blurEnabled) return 0
        return compositorBlurAvailable ? Math.max(0, blurRadius * 0.3) : blurRadius
    }

    Behavior on base {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on mantle {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on crust {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on surface {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on overlay {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on text {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on subtext {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on muted {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on accent {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on success {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on warning {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on error {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on radiusSmall {
        NumberAnimation { duration: root.animDurationFast; easing.type: root.animEasingEnter; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on radiusMedium {
        NumberAnimation { duration: root.animDurationFast; easing.type: root.animEasingEnter; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on radiusLarge {
        NumberAnimation { duration: root.animDurationFast; easing.type: root.animEasingEnter; easing.bezierCurve: root.animBezierCurve }
    }

    Behavior on transparencyLevel {
        NumberAnimation { duration: root.animDuration; easing.type: root.animEasingEnter; easing.bezierCurve: root.animBezierCurve }
    }

    // Smooth transitions for glass effects
    Behavior on sidebarOpacity {
        NumberAnimation { duration: root.animDuration; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }
    Behavior on contentOpacity {
        NumberAnimation { duration: root.animDuration; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }
    Behavior on toolbarOpacity {
        NumberAnimation { duration: root.animDuration; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }
    Behavior on glowOpacity {
        NumberAnimation { duration: root.animDuration; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }
    Behavior on blurRadius {
        NumberAnimation { duration: root.animDuration; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }
    Behavior on noiseOpacity {
        NumberAnimation { duration: root.animDuration; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }
    Behavior on saturation {
        NumberAnimation { duration: root.animDuration; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }
    Behavior on gradientColor {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }
    Behavior on glowColor {
        ColorAnimation { duration: root.animDurationSlow; easing.type: root.animEasingTransition; easing.bezierCurve: root.animBezierCurve }
    }

    function containerColor(color, defaultAlpha) {
        var strength = transparencyEnabled ? transparencyLevel : 0
        var alpha = 1 - strength * (1 - defaultAlpha)
        return Qt.rgba(color.r, color.g, color.b, alpha)
    }

    // Glass-aware container color: uses per-zone opacity when effects active
    function glassColor(color, zoneOpacity, fallbackAlpha) {
        if (hasEffects && transparencyEnabled) {
            return Qt.rgba(color.r, color.g, color.b, zoneOpacity)
        }
        return containerColor(color, fallbackAlpha)
    }

    readonly property color chromeBackground: hasEffects
        ? glassColor(mantle, toolbarOpacity, 0.75)
        : containerColor(mantle, 0.75)
    readonly property color contentBackground: hasEffects
        ? glassColor(base, contentOpacity, 0.65)
        : containerColor(base, 0.65)
    readonly property color overlayBackground: containerColor(mantle, 0.88)
    readonly property color popupBackground: containerColor(crust, 0.88)
    readonly property color sidebarBackground: hasEffects
        ? glassColor(crust, sidebarOpacity, 0.80)
        : containerColor(crust, 0.80)
}
