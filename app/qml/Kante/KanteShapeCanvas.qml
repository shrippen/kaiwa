import QtQuick
import "."

/**
 * A pannable, zoomable vector view (Kante 1.81, for Klappe): plans drawn from data, such as a
 * floor plan in metres. Presentational: the app owns `shapes` and answers the signals.
 *
 *   contentWidth, contentHeight   size of the plan in model units (x right, y down)
 *   gridStep                      grid in model units, 0 = none; left out while the lines would
 *                                 be closer than a few pixels
 *   shapes     [{ kind, points, radius, width, label, stroke, fill, text, key, selected, opacity }]
 *     kind     "line" (2 points), "polyline", "polygon" (closed), "circle" (points [cx, cy], radius)
 *     points   [x0, y0, x1, y1, …] in model units
 *     width    stroke width in model units, 0 = hairline (always at least one pixel)
 *     label    a colour of KanteStyle.labels, or "text", "muted", "accent", "focus", "warning",
 *              "ground" (the app's background: a sign on a filled shape, such as a dot in a
 *              filled circle, reads in every label colour)
 *     stroke   "solid" (default), "dashed", "dotted": a dash is a mark, not only a colour
 *     fill     "none" (default), "solid", "tint" (the colour at a fifth of its strength)
 *     text     small caption beside the shape, always at the same size on screen. A "\n" starts
 *              a new line (spaced 1.25 font sizes); the canvas does not wrap, the app does.
 *     captionSize  font size of the caption in model units: the caption then scales with the
 *              zoom, like the shape, and is left out below 4 px. Default: the small font, fixed.
 *     textAnchor  "center" (default) or "topLeft": a polygon's caption starts at its top left
 *              corner, with a small margin, instead of at its centre
 *     textRotation  degrees, clockwise like an image's rotation (default 0): all lines of the caption
 *              are drawn rotated about the anchor. With textAnchor "topLeft" the anchor is the
 *              polygon's first point, its top left corner when the app lists the corners from there
 *              clockwise, so a card turned by an angle carries its caption turned by the same angle.
 *     opacity  0..1 (default 1): fades fill, stroke, selection frame and caption; not the hit test
 *     key      id given back by the signals; selected: framed wider in the focus colour
 *     Drawn in order, so the last shape is on top (and wins the hit test).
 *   Image     { kind: "image", source, points: [x, y], width, height, rotation, opacity, key }
 *             a background such as a scanned plan. source is an absolute url or path:
 *             loadImage resolves a relative one against Kante, so pass Qt.resolvedUrl(...);
 *             points is the top left corner and width, height the size, both in model units;
 *             rotation in degrees, clockwise, about the top left corner; opacity 0..1 (default 1).
 *             Drawn in order like any shape. Loaded on first use and the view repaints when it
 *             arrives; a missing image draws nothing and is no error. It is background: the hit
 *             test never finds it (keyAt gives "" over it, shapeClicked and shapeDragged report
 *             what lies beneath, or empty ground), and it takes no label, fill, stroke or text.
 *   zoom       pixels per model unit; writing it zooms about the middle of the view
 *   fitOnLoad  fit() when the view gets its size, until the view is panned or zoomed
 *   fit()      the whole plan in view, centred
 *   mapToModel(px, py), mapFromModel(x, y)    {x, y} between view pixels and model units
 *   keyAt(px, py)             key of the topmost shape under a pixel, "" for none
 *   shapeClicked(key, x, y)   click without drag; key "" on empty ground; x, y in model units
 *   shapeDoubleClicked(key, x, y)   double click, as shapeClicked (a click comes first)
 *   shapeHovered(key, x, y)   the pointer moved; key "" when it is over nothing
 *   editing    the left button edits instead of panning: a drag on a shape or on empty ground
 *              reports shapeDragged; pan stays on the middle button and with space held
 *   shapeDragged(key, fromX, fromY, x, y, done)   in editing, a left drag: key of the shape
 *              it began on ("" on empty ground), where it began and where it is now (model
 *              units); done on release. The app moves or draws; the canvas changes nothing.
 *
 * Hit test: inside a polygon or circle that has a fill, or within a few pixels of an outline,
 * line or polyline (an unfilled polygon is only hit at its edge, so a room does not hide what
 * lies in it). Pan: middle button, space held, or left button on empty ground. Zoom: wheel or
 * pinch about the pointer. Keyboard (the item takes Tab): plus and minus zoom, 0 fits, arrows pan.
 *
 * The shapes are drawn with one Canvas, repainted when a property changes; 600 shapes take about
 * 5 ms to draw (offscreen), about 3 ms zoomed in, as shapes out of view are left out. The captions
 * are Text items over the canvas (one per caption, hidden out of view): the canvas draws no text,
 * a pan moves one container, and a caption costs the canvas nothing. A frame of that plan (272
 * captions) takes about 28 ms against 45 ms with the captions drawn into the canvas (offscreen,
 * loaded machine; qml/perf/tst_shapecanvas_perf.qml). A caption sized in model units is a whole
 * pixel size scaled to the exact one; glyphs are the Text item's, so their edges differ a little
 * from the canvas' own.
 */
FocusScope {
    id: root

    property real contentWidth: 100
    property real contentHeight: 100
    property real gridStep: 0
    property var shapes: []
    property real zoom: 1
    property bool fitOnLoad: true
    /** Name for screen readers. */
    property string accessibleName: "Plan"

    signal shapeClicked(string key, real x, real y)
    signal shapeDoubleClicked(string key, real x, real y)
    signal shapeHovered(string key, real x, real y)
    signal shapeDragged(string key, real fromX, real fromY, real x, real y, bool done)

    property bool editing: false

    // Where the model origin lies in the view.
    property real panX: 0
    property real panY: 0

    readonly property real _minZoom: 0.001
    readonly property real _maxZoom: 100000
    readonly property real _wheelStep: 1.0015
    readonly property real _keyZoom: 1.25
    readonly property real _dragSlop: KanteStyle.unit(4)
    readonly property real _reach: KanteStyle.unit(6)
    readonly property real _margin: KanteStyle.unit(24)
    // Distance between caption lines, in font sizes.
    readonly property real _lineSpacing: 1.25
    // A caption sized in model units is not drawn below this many pixels (unreadable).
    readonly property real _minCaptionPx: 4
    // The margin of a caption sized in model units, in its font sizes.
    readonly property real _captionGapEm: 0.3
    // A Text line sits this many font sizes lower than a canvas line set on its middle baseline; found
    // by laying both pictures over each other (Noto Sans).
    readonly property real _baselineEm: 0.15
    // Reach of a character for culling: wide glyphs (CJK) take up to about 1.1 em, so two em each.
    readonly property real _emsPerChar: 2
    readonly property real _minGridGap: KanteStyle.unit(6)
    readonly property real _tintAlpha: 0.2
    readonly property real _gridAlpha: 0.3

    // fit() while no one has moved the view: a layout settles the size after the first one.
    property bool _touched: false
    property bool _fitting: false
    property bool _ready: false
    property real _lastZoom: zoom
    property real _anchorX: width / 2
    property bool _anchored: false
    property real _anchorY: height / 2
    property bool _spaceDown: false
    property string _hoverKey: ""

    implicitWidth: KanteStyle.unit(480)
    implicitHeight: KanteStyle.unit(360)
    activeFocusOnTab: true
    clip: true

    Accessible.role: Accessible.Graphic
    Accessible.name: accessibleName

    /** View pixels to model units. */
    function mapToModel(px, py) {
        return { x: (px - panX) / zoom, y: (py - panY) / zoom }
    }

    /** Model units to view pixels. */
    function mapFromModel(x, y) {
        return { x: panX + x * zoom, y: panY + y * zoom }
    }

    /** The whole plan in view, centred. */
    function fit() {
        if (width <= 0 || height <= 0 || contentWidth <= 0 || contentHeight <= 0) {
            return
        }

        const room = Math.max(1, Math.min(_margin, width / 4, height / 4))
        const z = Math.min((width - 2 * room) / contentWidth, (height - 2 * room) / contentHeight)
        _fitting = true
        _lastZoom = z
        zoom = z
        panX = (width - contentWidth * z) / 2
        panY = (height - contentHeight * z) / 2
        _fitting = false
        _touched = false
        canvas.requestPaint()
    }

    // Zooms by `factor`, the model point under (px, py) stays under it.
    function _zoomAt(px, py, factor) {
        _anchorX = px
        _anchorY = py
        _anchored = true
        zoom = Math.max(_minZoom, Math.min(_maxZoom, zoom * factor))
        _anchored = false
    }

    // Any change of `zoom`, also an app's own: about the anchor, else about the middle.
    onZoomChanged: {
        if (_ready && !_fitting) {
            _touched = true
        }
        if (_lastZoom > 0 && zoom !== _lastZoom) {
            const ax = _anchored ? _anchorX : width / 2
            const ay = _anchored ? _anchorY : height / 2
            const k = zoom / _lastZoom
            panX = ax - (ax - panX) * k
            panY = ay - (ay - panY) * k
        }
        _lastZoom = zoom
        canvas.requestPaint()
    }

    // Space held while the focus leaves never sends its release.
    onActiveFocusChanged: {
        if (!activeFocus) {
            _spaceDown = false
        }
    }

    onPanXChanged: canvas.requestPaint()
    onPanYChanged: canvas.requestPaint()
    onShapesChanged: canvas.requestPaint()
    onGridStepChanged: canvas.requestPaint()
    onContentWidthChanged: canvas.requestPaint()
    onContentHeightChanged: canvas.requestPaint()
    onWidthChanged: _maybeFit()
    onHeightChanged: _maybeFit()
    Component.onCompleted: {
        _ready = true
        _fitCaptionRows()
        _maybeFit()
    }

    // A drag or a key moved the view.
    function _pan(dx, dy) {
        _touched = true
        panX += dx
        panY += dy
    }

    function _maybeFit() {
        if (fitOnLoad && !_touched) {
            fit()
        }
        canvas.requestPaint()
    }

    // Distance from (px, py) to the segment (ax, ay)-(bx, by).
    function _segDist(px, py, ax, ay, bx, by) {
        const dx = bx - ax
        const dy = by - ay
        const len2 = dx * dx + dy * dy
        const t = len2 > 0 ? Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / len2)) : 0
        return Math.hypot(px - (ax + t * dx), py - (ay + t * dy))
    }

    // Distance to the outline of a point list, closed or not.
    function _outlineDist(pts, mx, my, closed) {
        const n = Math.floor(pts.length / 2)
        let best = Infinity
        for (let i = 0; i + 1 < n; i++) {
            best = Math.min(best, _segDist(mx, my, pts[2 * i], pts[2 * i + 1], pts[2 * i + 2], pts[2 * i + 3]))
        }
        if (closed && n > 2) {
            best = Math.min(best, _segDist(mx, my, pts[2 * n - 2], pts[2 * n - 1], pts[0], pts[1]))
        }
        return best
    }

    // Even-odd test: is the model point inside the polygon.
    function _inPolygon(pts, mx, my) {
        const n = Math.floor(pts.length / 2)
        let inside = false
        for (let i = 0, j = n - 1; i < n; j = i++) {
            const xi = pts[2 * i]
            const yi = pts[2 * i + 1]
            const xj = pts[2 * j]
            const yj = pts[2 * j + 1]
            if ((yi > my) !== (yj > my) && mx < (xj - xi) * (my - yi) / (yj - yi) + xi) {
                inside = !inside
            }
        }
        return inside
    }

    // Is the model point (mx, my) on shape s; `reach` is the hit distance in model units.
    function _hits(s, mx, my, reach) {
        const pts = s.points || []
        const filled = s.fill === "solid" || s.fill === "tint"
        const half = (s.width || 0) / 2
        switch (s.kind) {
        case "image":
            // Background: never hit.
            return false
        case "circle":
            if (pts.length < 2) {
                return false
            }

            const d = Math.hypot(mx - pts[0], my - pts[1])
            const r = s.radius || 0
            return filled ? d <= r + reach : Math.abs(d - r) <= reach + half
        case "polygon":
            if (filled && _inPolygon(pts, mx, my)) {
                return true
            }
            return _outlineDist(pts, mx, my, true) <= reach + half
        case "line":
        case "polyline":
            return _outlineDist(pts, mx, my, false) <= reach + half
        }
        return false
    }

    /** The index of the topmost shape under a pixel, -1 for none. */
    function shapeAt(px, py) {
        const m = mapToModel(px, py)
        const reach = _reach / zoom
        for (let i = shapes.length - 1; i >= 0; i--) {
            if (_hits(shapes[i], m.x, m.y, reach)) {
                return i
            }
        }
        return -1
    }

    /** The key of the topmost shape under a pixel, "" for none. */
    function keyAt(px, py) {
        const i = shapeAt(px, py)
        if (i < 0) {
            return ""
        }

        // A key of 0 is a key.
        const key = shapes[i].key
        return key === undefined || key === null ? "" : String(key)
    }

    Keys.onPressed: function (event) {
        const step = KanteStyle.unit(40)
        event.accepted = true
        switch (event.key) {
        case Qt.Key_Plus:
        case Qt.Key_Equal:
            _zoomAt(width / 2, height / 2, _keyZoom)
            break
        case Qt.Key_Minus:
            _zoomAt(width / 2, height / 2, 1 / _keyZoom)
            break
        case Qt.Key_0:
            fit()
            break
        case Qt.Key_Left:
            _pan(step, 0)
            break
        case Qt.Key_Right:
            _pan(-step, 0)
            break
        case Qt.Key_Up:
            _pan(0, step)
            break
        case Qt.Key_Down:
            _pan(0, -step)
            break
        case Qt.Key_Space:
            _spaceDown = true
            break
        default:
            event.accepted = false
        }
    }
    Keys.onReleased: function (event) {
        if (event.key === Qt.Key_Space && !event.isAutoRepeat) {
            _spaceDown = false
        }
    }

    // The captions as plain data, worked out when the shapes change. `mx`, `my` is the anchor in
    // model units; the anchor moves by `r` model units and `gx`, `gy` gaps (a circle's caption
    // stands right of its edge, a line's right of and above its middle); `topLeft` hangs the
    // lines from the anchor, else they centre on it ("center" aligns each line as well).
    function _spots(list) {
        const out = []
        for (const s of list) {
            if (s.text && s.kind !== "image" && (s.points || []).length >= 2) {
                out.push(_spot(s, s.points))
            }
        }
        return out
    }

    function _spot(s, pts) {
        const c = { text: String(s.text), label: String(s.label), size: s.captionSize > 0 ? s.captionSize : 0,
                    rot: s.textRotation || 0, alpha: Number.isFinite(s.opacity) ? Math.max(0, Math.min(1, s.opacity)) : 1,
                    mx: pts[0], my: pts[1], r: 0, gx: 0, gy: 0, topLeft: false, center: false }
        if (s.kind === "circle") {
            c.r = s.radius || 0
            c.gx = 1
            return c
        }
        if (s.kind !== "polygon") {
            const n = pts.length
            c.mx = (pts[0] + pts[n - 2]) / 2
            c.my = (pts[1] + pts[n - 1]) / 2
            c.gx = 1
            c.gy = -1
            return c
        }

        const xs = pts.filter((v, i) => i % 2 === 0)
        const ys = pts.filter((v, i) => i % 2 === 1)
        c.topLeft = s.textAnchor === "topLeft"
        c.center = !c.topLeft
        if (c.center) {
            c.mx = (Math.min(...xs) + Math.max(...xs)) / 2
            c.my = (Math.min(...ys) + Math.max(...ys)) / 2
            return c
        }

        // A rotated caption turns about the corner the card's own turn leaves in place.
        c.mx = c.rot ? pts[0] : Math.min(...xs)
        c.my = c.rot ? pts[1] : Math.min(...ys)
        return c
    }
    readonly property var _captions: _spots(shapes)
    on_CaptionsChanged: _fitCaptionRows()

    // One row per caption. Rows are only added or removed at the end, so the Text items of the
    // others stay; an integer model would recreate them all when the count changes.
    ListModel { id: captionRows }
    readonly property var _noCaption: ({ text: "", label: "", size: 0, rot: 0, alpha: 0, mx: 0, my: 0, r: 0,
                                         gx: 0, gy: 0, topLeft: false, center: false })
    function _fitCaptionRows() {
        const n = _captions.length
        if (captionRows.count > n) {
            captionRows.remove(n, captionRows.count - n)
        }
        while (captionRows.count < n) {
            captionRows.append({})
        }
    }
    readonly property real _fontPx: KanteStyle.fontPixels(KanteStyle.smallFont)

    // The theme and the font repaint the canvas.
    readonly property var _look: [KanteStyle.backgroundColor, KanteStyle.textColor, KanteStyle.mutedTextColor,
        KanteStyle.accentColor, KanteStyle.focusColor, KanteStyle.warningColor, KanteStyle.ruleColor,
        KanteStyle.smallFont]
    on_LookChanged: canvas.requestPaint()

    Canvas {
        id: canvas
        anchors.fill: parent

        // Image sources already asked for.
        property var _asked: ({})

        // Per paint: colours by label name, and what the context was last given, so each
        // value is worked out once and a context state is set only when it changes.
        property var _colors: ({})
        property var _tints: ({})
        property string _dashKey: ""

        // The colour of a label name, worked out once per paint.
        function _color(name) {
            let c = _colors[name]
            if (c === undefined) {
                c = KanteStyle.namedColor(name)
                _colors[name] = c
            }
            return c
        }

        function _tint(name) {
            let c = _tints[name]
            if (c === undefined) {
                c = Qt.alpha(_color(name), root._tintAlpha)
                _tints[name] = c
            }
            return c
        }

        // Does shape s, stroked `lineW` wide in view pixels, reach into the view.
        function _onScreen(s, lineW) {
            const pts = s.points || []
            let x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity
            for (let i = 0; i + 1 < pts.length; i += 2) {
                x0 = Math.min(x0, pts[i])
                x1 = Math.max(x1, pts[i])
                y0 = Math.min(y0, pts[i + 1])
                y1 = Math.max(y1, pts[i + 1])
            }
            const r = s.kind === "circle" ? s.radius || 0 : 0
            const reach = lineW / 2 + KanteStyle.unit(2) + 1
            return root.panX + (x1 + r) * root.zoom + reach >= 0
                && root.panX + (x0 - r) * root.zoom - reach <= width
                && root.panY + (y1 + r) * root.zoom + reach >= 0
                && root.panY + (y0 - r) * root.zoom - reach <= height
        }
        onImageLoaded: requestPaint()

        // Strokes and fills of one shape, in view pixels.
        function _trace(ctx, s) {
            const pts = s.points || []
            if (s.kind === "circle") {
                if (pts.length < 2) {
                    return false
                }

                ctx.beginPath()
                ctx.arc(root.panX + pts[0] * root.zoom, root.panY + pts[1] * root.zoom, (s.radius || 0) * root.zoom, 0, 2 * Math.PI)
                return true
            }
            if (pts.length < 4) {
                return false
            }

            ctx.beginPath()
            for (let i = 0; i + 1 < pts.length; i += 2) {
                const x = root.panX + pts[i] * root.zoom
                const y = root.panY + pts[i + 1] * root.zoom
                if (i === 0) {
                    ctx.moveTo(x, y)
                } else {
                    ctx.lineTo(x, y)
                }
            }
            if (s.kind === "polygon") {
                ctx.closePath()
            }
            return true
        }

        // An image shape: top left corner, size in model units, turned about the corner.
        function _image(ctx, s) {
            const pts = s.points || []
            if (!s.source || pts.length < 2) {
                return
            }

            const src = String(s.source)
            // Asked once; onImageLoaded repaints. An image Qt already holds is ready at once
            // and sends no signal, so look again right after asking.
            if (!isImageLoaded(src) && !_asked[src]) {
                _asked[src] = true
                loadImage(src)
            }
            if (!isImageLoaded(src)) {
                return
            }

            ctx.save()
            ctx.globalAlpha = _alpha(s)
            const a = (s.rotation || 0) * Math.PI / 180
            ctx.transform(Math.cos(a), Math.sin(a), -Math.sin(a), Math.cos(a),
                          root.panX + pts[0] * root.zoom, root.panY + pts[1] * root.zoom)
            ctx.drawImage(src, 0, 0, (s.width || 0) * root.zoom, (s.height || 0) * root.zoom)
            ctx.restore()
        }

        // A shape's opacity, 0..1; 1 when not given.
        function _alpha(s) {
            return Number.isFinite(s.opacity) ? Math.max(0, Math.min(1, s.opacity)) : 1
        }

        function _dash(ctx, stroke) {
            const key = stroke || "solid"
            if (key === _dashKey) {
                return
            }

            _dashKey = key
            switch (stroke) {
            case "dashed":
                ctx.setLineDash([KanteStyle.unit(8), KanteStyle.unit(5)])
                return
            case "dotted":
                ctx.setLineDash([KanteStyle.unit(2), KanteStyle.unit(4)])
                return
            }
            ctx.setLineDash([])
        }

        function _grid(ctx) {
            const step = root.gridStep * root.zoom
            if (root.gridStep <= 0 || step < root._minGridGap) {
                return
            }

            const w = root.contentWidth * root.zoom
            const h = root.contentHeight * root.zoom
            ctx.strokeStyle = Qt.alpha(KanteStyle.ruleColor, root._gridAlpha)
            ctx.lineWidth = 1
            ctx.setLineDash([])
            ctx.beginPath()
            for (let gx = 0; gx <= root.contentWidth + 1e-9; gx += root.gridStep) {
                const x = Math.round(root.panX + gx * root.zoom) + 0.5
                ctx.moveTo(x, root.panY)
                ctx.lineTo(x, root.panY + h)
            }
            for (let gy = 0; gy <= root.contentHeight + 1e-9; gy += root.gridStep) {
                const y = Math.round(root.panY + gy * root.zoom) + 0.5
                ctx.moveTo(root.panX, y)
                ctx.lineTo(root.panX + w, y)
            }
            ctx.stroke()
        }

        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()
            ctx.fillStyle = KanteStyle.backgroundColor
            ctx.fillRect(0, 0, width, height)

            _grid(ctx)

            // The edge of the plan.
            ctx.strokeStyle = KanteStyle.ruleColor
            ctx.lineWidth = 1
            ctx.setLineDash([])
            ctx.strokeRect(root.panX, root.panY, root.contentWidth * root.zoom, root.contentHeight * root.zoom)

            ctx.lineJoin = "round"
            ctx.lineCap = "round"

            _colors = ({})
            _tints = ({})
            _dashKey = "solid"

            const frame = KanteStyle.unit(2)
            for (const s of root.shapes) {
                if (s.kind === "image") {
                    _image(ctx, s)
                    continue
                }

                const color = _color(s.label)
                const lineW = Math.max(1, (s.width || 0) * root.zoom)
                if (!_onScreen(s, s.selected === true ? lineW + 2 * frame : lineW) || !_trace(ctx, s)) {
                    continue
                }

                ctx.globalAlpha = _alpha(s)
                if (s.fill === "solid" || s.fill === "tint") {
                    ctx.fillStyle = s.fill === "solid" ? color : _tint(s.label)
                    ctx.fill()
                }
                // A selected shape sits in a wider frame of the focus colour.
                if (s.selected === true) {
                    _dashKey = "solid"
                    ctx.setLineDash([])
                    ctx.strokeStyle = KanteStyle.focusColor
                    ctx.lineWidth = lineW + 2 * frame
                    ctx.stroke()
                }
                _dash(ctx, s.stroke)
                ctx.strokeStyle = color
                ctx.lineWidth = lineW
                ctx.stroke()
            }

            ctx.globalAlpha = 1
        }
    }

    // Captions are Text items over the canvas, so a pan moves one container and a caption costs the
    // canvas nothing. Each sits in an item at its anchor (turned and scaled about it); fractional
    // font sizes are a whole pixel size scaled by `k`. Out of view ones are hidden, as they were
    // left out of the drawing. No mouse input: the MouseArea below takes it all.
    Item {
        id: captionLayer
        x: root.panX
        y: root.panY
        Accessible.ignored: true

        Repeater {
            model: captionRows

            Item {
                id: spot

                required property int index
                // Fewer captions: until its row goes, a delegate at the end reads an empty one.
                readonly property var c: root._captions[index] ?? root._noCaption
                readonly property bool sized: c.size > 0
                // Font size on screen; 0 when too small to read.
                readonly property real px: !sized ? root._fontPx : (c.size * root.zoom < root._minCaptionPx ? 0 : c.size * root.zoom)
                readonly property real dp: sized ? Math.max(1, Math.round(px)) : px
                readonly property real k: px / dp
                readonly property real gap: sized ? px * root._captionGapEm : KanteStyle.unit(4)
                readonly property real lineH: dp * root._lineSpacing
                readonly property int rows: c.text.split("\n").length - 1
                // Anchor in view pixels, then where the first line's middle lies from it.
                readonly property real vx: root.panX + root.zoom * (c.mx + c.r) + c.gx * gap
                readonly property real vy: root.panY + root.zoom * c.my + c.gy * gap
                readonly property real first: c.topLeft ? gap + px / 2 : -rows * px * root._lineSpacing / 2
                readonly property real reach: c.text.length * root._emsPerChar * px
                readonly property real turnReach: reach + rows * px * root._lineSpacing + gap + 2 * px
                readonly property bool out: c.rot
                    ? vx + turnReach < 0 || vx - turnReach > root.width || vy + turnReach < 0 || vy - turnReach > root.height
                    : vx + (c.topLeft ? gap : 0) + reach < 0 || vx + (c.topLeft ? gap : 0) - reach > root.width
                      || vy + first + rows * px * root._lineSpacing + px < 0 || vy + first - px > root.height

                visible: px > 0 && !out
                x: root.zoom * (c.mx + c.r) + c.gx * gap
                y: root.zoom * c.my + c.gy * gap
                rotation: c.rot
                scale: k
                transformOrigin: Item.TopLeft
                opacity: c.alpha

                Text {
                    x: (c.topLeft ? spot.gap / spot.k : 0) - (c.center ? implicitWidth / 2 : 0)
                    y: (c.topLeft ? spot.gap / spot.k + spot.dp / 2 : -spot.rows * spot.lineH / 2) - spot.lineH / 2 - spot.dp * root._baselineEm
                    text: spot.c.text
                    textFormat: Text.PlainText
                    color: KanteStyle.readable(KanteStyle.namedColor(spot.c.label), KanteStyle.backgroundColor)
                    font: spot.sized ? Qt.font({ family: KanteStyle.smallFont.family, pixelSize: spot.dp }) : KanteStyle.smallFont
                    lineHeight: spot.lineH
                    lineHeightMode: Text.FixedHeight
                    horizontalAlignment: spot.c.center ? Text.AlignHCenter : Text.AlignLeft
                    Accessible.ignored: true
                }
            }
        }
    }

    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
        cursorShape: panning ? Qt.ClosedHandCursor : (root._spaceDown ? Qt.OpenHandCursor : Qt.ArrowCursor)

        property bool panning: false
        property bool moved: false
        // In editing: a left drag the app gets, and the shape it began on.
        property bool dragging: false
        property string dragKey: ""
        // Where the drag began, in model units: fixed at the press, whatever the view does.
        property real dragFromX: 0
        property real dragFromY: 0
        property real lastModelX: 0
        property real lastModelY: 0
        property real lastX: 0
        property real lastY: 0
        property real startX: 0
        property real startY: 0

        function _model(mx, my) {
            return root.mapToModel(mx, my)
        }

        onPressed: function (mouse) {
            root.forceActiveFocus()
            lastX = startX = mouse.x
            lastY = startY = mouse.y
            moved = false
            // Middle button and space always pan; the left button pans from empty ground,
            // unless the app edits: then it gets the drag.
            const forced = mouse.button === Qt.MiddleButton || root._spaceDown
            panning = forced || (!root.editing && root.shapeAt(mouse.x, mouse.y) < 0)
            dragging = root.editing && !forced
            dragKey = dragging ? root.keyAt(mouse.x, mouse.y) : ""
            const from = _model(startX, startY)
            dragFromX = lastModelX = from.x
            dragFromY = lastModelY = from.y
        }
        function _drag(mx, my, done) {
            const m = _model(mx, my)
            lastModelX = m.x
            lastModelY = m.y
            root.shapeDragged(dragKey, dragFromX, dragFromY, m.x, m.y, done)
        }
        onPositionChanged: function (mouse) {
            if (!pressed) {
                const k = root.keyAt(mouse.x, mouse.y)
                const m = _model(mouse.x, mouse.y)
                // Over a shape the position goes on; over nothing only the change is reported.
                if (k !== "" || root._hoverKey !== "") {
                    root.shapeHovered(k, m.x, m.y)
                }
                root._hoverKey = k
                return
            }
            if (Math.hypot(mouse.x - startX, mouse.y - startY) > root._dragSlop) {
                moved = true
            }
            if (dragging && moved) {
                _drag(mouse.x, mouse.y, false)
                return
            }
            if (!panning || !moved) {
                return
            }

            root._pan(mouse.x - lastX, mouse.y - lastY)
            lastX = mouse.x
            lastY = mouse.y
        }
        onReleased: function (mouse) {
            if (dragging && moved) {
                _drag(mouse.x, mouse.y, true)
            }
            panning = false
            dragging = false
        }
        // A pinch or the window took the press over: end the drag where it was last seen.
        onCanceled: {
            if (dragging && moved) {
                root.shapeDragged(dragKey, dragFromX, dragFromY, lastModelX, lastModelY, true)
            }
            panning = false
            dragging = false
        }
        onExited: {
            if (root._hoverKey !== "") {
                root.shapeHovered("", 0, 0)
                root._hoverKey = ""
            }
        }
        onClicked: function (mouse) {
            if (moved || mouse.button !== Qt.LeftButton) {
                return
            }

            const m = _model(mouse.x, mouse.y)
            root.shapeClicked(root.keyAt(mouse.x, mouse.y), m.x, m.y)
        }
        onDoubleClicked: function (mouse) {
            if (mouse.button !== Qt.LeftButton) {
                return
            }

            // The second click is reported as before; the double click follows it.
            const m = _model(mouse.x, mouse.y)
            const key = root.keyAt(mouse.x, mouse.y)
            root.shapeClicked(key, m.x, m.y)
            root.shapeDoubleClicked(key, m.x, m.y)
        }
        onWheel: function (wheel) {
            root._zoomAt(wheel.x, wheel.y, Math.pow(root._wheelStep, wheel.angleDelta.y))
        }
    }

    PinchHandler {
        id: pinch
        target: null
        property real last: 1
        onActiveChanged: last = 1
        onActiveScaleChanged: {
            if (!active) {
                return
            }

            root._zoomAt(centroid.position.x, centroid.position.y, activeScale / last)
            last = activeScale
        }
    }

    // Focus frame.
    Rectangle {
        anchors.fill: parent
        color: "transparent"
        border.width: root.activeFocus ? KanteStyle.unit(2) : 0
        border.color: KanteStyle.focusColor
    }
}
