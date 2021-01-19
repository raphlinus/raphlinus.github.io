---
layout: post
title:  "Critique of Oklab"
date:   2021-01-18 14:39:42 -0700
categories: [color]
---
<script type="text/x-mathjax-config">
	MathJax.Hub.Config({
		tex2jax: {
			inlineMath: [['$', '$']]
		}
	});
</script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.0/MathJax.js?config=TeX-AMS-MML_HTMLorMML" type="text/javascript"></script>
Björn Ottosson recenty published a blog post introducing [Oklab]. The blog claimed that Oklab is a better perceptual color space than what came before. It piqued my interest, and I wanted to see for myself.

In exploring perceptual color spaces, I find an interactive gradient tool to be invaluable, so I've reproduced one here:

<style>
    .gradient {
        display: flex;
        align-items: center;
        justify-content: center;
        flex-wrap: wrap;
    }
    .gradname {
        width: 5em;
    }
    .sliders {
        display: flex;
        flex-wrap: wrap;
        justify-content: space-evenly;
    }
    .cluster {
        margin: 10px;
    }
    .axis {
        display: flex;
        flex-wrap: wrap;
    }
    .axis div:nth-child(1) {
        width: 2em;
    }
    .axis div:nth-child(2) {
        width: 2em;
    }
    .buttonrow {
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
        margin-top: 5px;
        margin-bottom: 5px;
    }
    .buttonrow button {
        margin: 0 5px;
    }
    .quantize {
        display: flex;
        flex-wrap: wrap;
        margin-bottom: 10px;
        justify-content: center;
    }
    .quantize div {
        margin: 0 10px;
    }
</style>

<div class="gradients">

<div class="gradient">
<div class="gradname">sRGB</div>
<div><canvas width="480" height="50" id="c0"></canvas></div>
</div>
<div class="gradient">
<div class="gradname">CIELAB</div>
<div><canvas width="480" height="50" id="c1"></canvas></div>
</div>
<div class="gradient">
<div class="gradname">IPT</div>
<div><canvas width="480" height="50" id="c2"></canvas></div>
</div>
<div class="gradient">
<div class="gradname">Oklab</div>
<div><canvas width="480" height="50" id="c3"></canvas></div>
</div>
<div class="gradient">
<div class="gradname">ICtCp</div>
<div><canvas width="480" height="50" id="c4"></canvas></div>
</div>
</div>

<!--Jerry-rigged color picker-->
<div class="buttonrow">
    <button id="randomize" type="button">Random</button>
    <button id="q0"><span style="background: #00f">&#x2003;</span><span style="background: #fff">&#x2003;</span></button>
    <button id="q1"><span style="background: #000">&#x2003;</span><span style="background: #fff">&#x2003;</span></button>
    <button id="q2"><span style="background: #001">&#x2003;</span><span style="background: #fff">&#x2003;</span></button>
    <button id="q3"><span style="background: #00f">&#x2003;</span><span style="background: #ff0">&#x2003;</span></button>
    <button id="q4"><span style="background: #f00">&#x2003;</span><span style="background: #00f">&#x2003;</span></button>
    <button id="q5"><span style="background: #f00">&#x2003;</span><span style="background: #0f0">&#x2003;</span></button>
</div>
<div class="sliders">
<div class="cluster">
<div class="axis">
<div>R</div>
<div id="ro1"></div>
<div><input id="r1" type="range" min="0" max="255" value="0"></div>
</div>
<div class="axis">
<div>G</div>
<div id="go1"></div>
<div><input id="g1" type="range" min="0" max="255" value="0"></div>
</div>
<div class="axis">
<div>B</div>
<div id="bo1"></div>
<div><input id="b1" type="range" min="0" max="255" value="255"></div>
</div>
</div>

<div class="cluster">
<div class="axis">
<div>R</div>
<div id="ro2"></div>
<div><input id="r2" type="range" min="0" max="255" value="255"></div>
</div>
<div class="axis">
<div>G</div>
<div id="go2"></div>
<div><input id="g2" type="range" min="0" max="255" value="255"></div>
</div>
<div class="axis">
<div>B</div>
<div id="bo2"></div>
<div><input id="b2" type="range" min="0" max="255" value="255"></div>
</div>
</div>
</div>
<div class="quantize">
<div>Quantize</div>
<div>
<input id="quant" type="range" min="0" max="1" value="1" step="any" />
</div>
</div>

## Why (and when) a perceptual color space?

Most image processing is done using a device color space (most often [sRGB]), and most libraries and interfaces expose that color space. Even when an image editing tool or a standard (such as CSS) exposes other color spaces, it's still most common to use the device color space. But for some use cases, a perceptual color space can give better results.

Basically *the* central question of color theory is how colors (in the physical sense) are perceived. The trichromacy assumption basically groups colors into equivalence classes in a three-dimensional space, and display devices generally produce colors by mixing three additive primaries: the familiar red, green, and blue.

In an ideal perceptual color space, the distance of two points in the space would correlate strongly with the *perception* of color difference. Put another way, all pairs separated by a "just noticeable difference" would be separated by an equal distance.

As it turns out, such a thing is no more possible than flattening an orange peel, because color perception is inherently [non-euclidean][Non-euclidean structure of spectral color space]. To put it simply, our eyes are more sensitive to small changes in hue than small changes in lightness or color saturation.

Even so, like map projections, it is possible to make a color space that approximates perceptual uniformity and is useful for various tasks. One of these, a primary focus of this blog post, is smoother gradients.

Gradients are of course very similar to color scales, and a good perceptual color space can be used as the basis for those. An even better approach is to use a real color appearance model, as was done in [A Better Default Colormap for Matplotlib], and is well explained and motivated in that video (it contains a brief introduction to color science as well).

A major application of perceptual color spaces is image manipulation, especially changing the color saturation of an image. This is a particular place where hue uniformity is important, as you don't want hues to shift. Also, prediction of lightness is especially important when transforming a color image to black and white.

Perceptual color spaces are also a good basis for programmatic manipulation of color palettes, for example to create sets of colors in particular relation to each other, or to derive one of dark and light mode from the other. For example, the normal/hover/active/disabled states of a button may be different lightness and saturation values of the same hue, so hue shift would be undesirable. In particular, I think a future evolution of the standard [Blend modes] should have at least the option to do the blending in a high quality perceptual color space.

And, as mentioned by Björn, a color picker widget can benefit from a good perceptual space. It should be possible to adjust lightness and saturation without affecting hue, in particular.

Much of the literature on perceptual color spaces is geared to image compression, with two primary motivations. First, as compression adds errors, you generally want those errors distributed evenly in perceptual space; it wouldn't be good at all to have artifacts that appear more prominently in areas of a particular shade. Second, good compression depends on a clean separation of lightness and chroma information, as the latter can be compressed better.

All that said, there are definitely cases where you do *not* want to use a perceptual space. Generally for image filtering, antialiasing, and alpha compositing, you want to use a linear space (though there are subtleties here). And there are even some cases you want to use a device space, as the device gamut is usually nice cube there, while it has quite the complex shape in other color spaces.

## Focus on gradients

This blog post will use gradients as the primary lens to study perceptual color spaces. They are a very sensitive instrument for certain flaws (especially lack of hue uniformity), and a useful goal in and of itself.

## No one true gamma

The transfer function for neutral colors is the literal backbone of any color space. Commonly referred to as "gamma," it is one of the most commonly misunderstood topics in computer graphics. [Poynton's thesis] is a definitive account, and I refer the interested reader there, but will try to summarize the main points.

An ideal transfer function for gradients will have perceptually equal steps from black to white. In *general,* the transfer function in CIELAB is considered close to perceptually uniform, but as always in color perception, the truth is a bit more complicated.

In particular, perception depends on viewing conditions. That includes the ambient light, but also the surround; the same gradient surrounded by white will appear darker than when surrounded by black. For an extremely compelling demonstration of the power of surround to affect the perception of lightness, see [Akiyoshi's illusion pages], for example [this one](http://www.psy.ritsumei.ac.jp/~akitaoka/light2e.html).

Another complication is that the light received by the eye includes so-called "veiling glare," a fraction of ambient light reflected by the monitor because its black is not a perfect absorber (veiling glare is much less of a problem in movie-like conditions).

The actual CIELAB transfer function is not a perfect cube-root rule, but rather contains a linear segment in the near-black region (the sRGB transfer function is similar but has different parameters). This segment increases perceptual uniformity of ramps in the presence of veiling glare, and also makes the transform robustly invertible using lookup tables.

There is good science on how perception varies with viewing conditions, and the [CIECAM] color appearance model has parameters that can fine-tune it when these are known. But in practice, viewing conditions are not known, and the best approach is to adopt a best guess or compromise.

### HDR

HDR is a different story, and I need to go into it to explain the [ICtCp] color space.

In standard dynamic range, you basically assume the visual system is adapted to a particular set of viewing conditions (in fact, [sRGB] specifies an exact set of viewing conditions, including monitor brightness, white point, and room lighting). A perceptually uniform gradient from black to white is also useful for image coding, because if you set number of steps so that each individual step is *just* imperceptible, it uses a minimum number of bits for each sample while faithfully rendering the image without artifacts. And in sRGB, 256 level is just barely enough for most uses, though steps are often visible when displaying gradients, the case where the eye is most sensitive to quantization errors.

In HDR, however, this approach doesn't quite work. Because of the wider range of brightness values from the display device, and also weaker assumptions about the viewing conditions (darkened rooms are common for movie viewing), the human visual system might at any time be adapted to quite light or quite dark viewing conditions. In the latter case, it would be sensitive to much finer gradations in near-black shades than when adapted to lighter conditions, and a similar situation is true the other way around. If tuned for any one single brightness level, results will be good when adaptation matches, but poor otherwise.

Thus, HDR uses a different approach. It uses a model (known as the Barten model, and shown in Figure 4.6 of [Poynton's thesis]) of the minimum contrast step perceptible at each brightness level, over all possible adaptation conditions. The goal is to determine a sequence of steps so that each step is just under the threshold of what's perceptible under *any* viewing conditions.

The SMPTE ST 2084 transfer function is basically a mathematical curve-fit to the empirical Barten model, and has the property that with 12 bits of code words, each step is just under 0.9 of the minimum perceptual difference as predicted by the Barten model, across a range from 0.001 to 10,000 nits of brightness (7 orders of magnitude). There's lots more detail and context in the presentation [A Perceptual EOTF for Extended
Dynamic Range Imagery] (PDF).

That said, though it's sophisticated and an excellent fit to the empirical Barten curve, it is *not* perceptually uniform at any one particular viewing condition. In particular, a ramp of the ST 2084 curve will dwell far too long near-black (representing a range that would be more visible in dark viewing conditions). To see this for yourself, try the black+white button in the interactive explorer above.

### A comparison of curves

We can basically place curves on a scale from "way too dark" (ST 2084) to "way too light" (linear light), with all the others in between. CIELAB is a pretty good median (though this may express my personal preference), with IPT a bit lighter and Oklab a bit darker.

<img src="/assets/colorspace_transfer_functions.png" width="575" style="margin: auto; display: block;" />

I found Björn's arguments in favor of pure cube root to be not entirely compelling, but this is perhaps an open question. Both CIELAB and sRGB use a finite-derivative region near black. Is it important to limit derivatives for more accurate LUT-based calculation? Perhaps in 2021, we will almost always prefer ALU to LUT. The conditional part is also not ideal, especially on GPUs, where branches can hurt performance. I personally would explore transfer functions of the form $f(x) = a + (b + cx)^\gamma$, constrained so $f(0) = 0$ and $f(1) = 1$, as these are GPU-friendly and have smooth derivatives. The XYB color space used in JPEG XL apparently uses a bias rather than a piecewise linear region, as well. (Source: [HN thread on Oklab], as I wasn't easily able to find a document)

## The Lab/IPT architecture

CIELAB, IPT, ICtCp, and Oklab all share a simple architecture: a 3x3 matrix, a nonlinear function, and another 3x3 matrix. In addition to being simple, this architecture is easily invertible. Many other color spaces are considerably more complex, with CIECAM as an especially bad offender.

<img src="/assets/ipt_flow.svg" alt="flow diagram of IPT architecture" style="margin: auto; display: block;" />

The main difference between the various color spaces in this architecture is the nonlinear function, which determines the black-to-white ramp as discussed above. Once that is in place, there is a relatively small number of remaining parameters. Those can be optimized, either by hand or using an automated optimizer trying to minimize a cost function.

In particular, [ICtCp] was derived from the ST 2084 transfer function, and then optimized for hue linearity and good lightness prediction. It's important to note, good lightness prediction in an HDR context does *not* mean that the lightness steps are perceptually uniform, but that colors with the same reported lightness have the same perceived lightness. ICtCp does well in the latter criterion, but not so much the former; it's fundamentally in tension with a color space suitable for HDR.

## Comparisons


### Hue linearity

On hue linearity, my evaluation is that IPT, ICtCp, and Oklab are all quite good, and a huge improvement over CIELAB. This is not terribly surprising, as they are all optimized for hue linearity with respect to the Ebner-Fairchild data set, which is of very high quality.

#### Where does the shift in blue come from?

It's always been a bit of a mystery to me *why* CIELAB has such a bad hue shift in the blue range. I think some of it is just bad tuning of the matrix parameters, but there is an even deeper issue. Additive mixing of deep blue and white creates a hue shift towards purple. James Clerk Maxwell would have been able to observe this in spinning top experiments, had he mixed a sufficiently strong blue; I'm not sure exactly when this was first noted.

More recent research indicates that changing the *bandwidth* of a gaussian spectrum, but retaining its peak wavelength, tends to change the perceived color saturation while retaining hue constancy. The authors of [Using Gaussian Spectra to Derive a Hue-linear Color Space] observe this and tries to use the fact to derive a hue-linear color space (very similar to IPT as well) from first principles, rather than experimental data. The results are mixed; the results are pretty good but not perfect, perhaps illustrating that an optimization process is always very sensitive to flaws in the optimization criterion. In any case, I found the paper to provide intuitive insight into why this problem is not so simple, and in particular why additive light mixing with neutral is not hue-preserving.

### Lightness

On lightness, the Oklab blog argues more accurate predictions than IPT. I was initially skeptical of this claim (it's not entirely consistent with similar claims in [Ebner's thesis][Fritz Ebner's thesis], see Figures 69 and 71 in particular), but on closer examination I agree. The following images should help you verify this claim yourself:

<img src="/assets/iso_luma_ipt.png" width="350" alt="iso-luma patches in IPT" />
<img src="/assets/iso_luma_oklab.png" width="350" alt="iso-luma patches in Oklab" />

The first is a collection of color patches with the same lightness (I) value in IPT, and the second with the same lightness (L) in Oklab. To my eyes, the second has more uniform lightness, while in IPT blues are too dark and yellow-greens are too light.

It's also possible to evaluate this claim objectively. The L* axis in CIELAB is widely agreed to predict lightness. Thus, deviations from it are a bad sign. The plots below show a scatterplot of random colors, with CIELAB L* on the horizontal axis, and the lightness axis of IPT and Oklab on the vertical axis:

<img src="/assets/ipt_l_scatter.png" width="350" alt="lightness scatterplot of IPT vs CIELAB" />
<img src="/assets/oklab_l_scatter.png" width="350" alt="lightness scatterplot of Oklab vs CIELAB" />

As can be seen, Oklab correlates *much* more strongly with CIELAB on the lightness scale, while IPT has considerable variation. I was also interested to see that ICtCp correlates strongly with CIELAB as well, though there's a pronounced nonlinearity due to the transfer function.

<img src="/assets/ictcp_l_scatter.png" width="350" alt="lightness scatterplot of ICtCp vs CIELAB" style="margin: auto; display: block;" />

Differences in lightness don't have a huge effect on gradients, but they do affect image processing operations such as changing saturation. Thus, I wouldn't recommend IPT as a color space for these operations, and am more comfortable recommending Oklab.

### Chroma

I didn't evaluate this claim as carefully, in part because its relevance to high quality gradients is limited. *Relative* changes in chroma are of course very important, but the *absolute* chroma value ascribed to highly saturated colors matters little to gradients. Even so, the quantitative data suggest that Oklab is a more accurate predictor, and it's easy to believe, as IPT wasn't carefully optimized for this criterion.

## Conclusions

A good perceptual color space can make higher quality, more uniform gradients. While CIELAB is popular and well-known, its hue shift is a problem. The original IPT color space was a great advancement: it has tremendously better hue constancy, while retaining the same simple structure. That said, its predictions of lightness and chroma are less highly tuned.

The desirable properties of IPT inspired a family of IPT-like color spaces, with the main difference being the choice of transfer function. Obviously the family includes ICtCp, which uses a transfer function optimized for HDR (but which, sadly, makes it less suitable for general purpose use). The modern recipe for an IPT-like color space is to choose a transfer function, then optimize the matrix parameters for hue linearity and accurate prediction of lightness and chroma. Oklab is basically the result of applying that recipe, starting at the cube-root transfer function.

For most applications where CIELAB is used today, both IPT and Oklab are better alternatives. The choice between them is a judgment call, though Oklab does have better prediction of chroma and lightness.

Is an even better color space possible? I think so. I personally would like to see a transfer function with a little more contrast in the near-black region (closer to CIELAB), but this is something of a judgment call. I also think it's possible to optimize on the basis of higher quality data; indexing off an existing color space retains any flaws in that space. Perhaps the biggest concern is that there is no one clear contender for a post-CIELAB standard. Another spin on Oklab with even higher quality data, and a consensus-building process, could be exactly that.

In the meantime, I can highly recommend Oklab for tasks such as making better gradients.

This blog post benefitted greatly from conversations with Björn Ottson and Jacob Rus, though of course my mistakes are my own.

[colour-science]: https://www.colour-science.org/
[ICtCp]: https://en.wikipedia.org/wiki/ICtCp
[Fritz Ebner's thesis]: https://www.researchgate.net/publication/221677980_Development_and_Testing_of_a_Color_Space_IPT_with_Improved_Hue_Uniformity
[Poynton's thesis]: http://poynton.ca/PDFs/Poynton-2018-PhD.pdf
[handprint.com]: https://handprint.com/HP/WCL/wcolor.html
[Oklab]: https://bottosson.github.io/posts/oklab/
[A Better Default Colormap for Matplotlib]: https://www.youtube.com/watch?v=xAoljeRJ3lU
[CIECAM]: https://en.wikipedia.org/wiki/CIECAM02
[Blend modes]: https://en.wikipedia.org/wiki/Blend_modes
[A Perceptual EOTF for Extended Dynamic Range Imagery]: https://www.avsforum.com/attachments/smpte-2014-05-06-eotf-miller-1-2-handout-pdf.1347114/
[Using Gaussian Spectra to Derive a Hue-linear Color Space]: https://doi.org/10.2352/J.Percept.Imaging.2020.3.2.020401
[HN thread on Oklab]: https://news.ycombinator.com/item?id=25525726
[Non-euclidean structure of spectral color space]: https://www.researchgate.net/publication/2900785_Non-Euclidean_Structure_of_Spectral_Color_Space
[sRGB]: https://en.wikipedia.org/wiki/SRGB
[Akiyoshi's illusion pages]: http://www.ritsumei.ac.jp/~akitaoka/index-e.html

<script>
// The following code is licensed under Apache-2.0 license as indicated in
// the "About" page of this blog.
function lerp(a, b, t) {
    if (Array.isArray(a)) {
        var result = [];
        for (let i = 0; i < a.length; i++) {
            result.push(lerp(a[i], b[i], t));
        }
        return result;
    } else {
        return a + (b - a) * t;
    }
}
function mat_vec_mul(m, v) {
    var result = [];
    for (row of m) {
        let sum = 0;
        for (let i = 0; i < row.length; i++) {
            sum += row[i] * v[i];
        }
        result.push(sum);
    }
    return result;
}
// Argument is in range 0..1
function srgb_eotf(u) {
    if (u < 0.04045) {
        return u / 12.92;
    } else {
        return Math.pow((u + 0.055) / 1.055, 2.4);
    }
}
function srgb_eotf_inv(u) {
    if (u < 0.0031308) {
        return u * 12.92;
    } else {
        return 1.055 * Math.pow(u, 1/2.4) - 0.055;
    }
}
// Source: Wikipedia sRGB article, rounded to 4 decimals
const SRGB_TO_XYZ = [
    [0.4124, 0.3576, 0.1805],
    [0.2126, 0.7152, 0.0722],
    [0.0193, 0.1192, 0.9505]
];
const XYZ_TO_SRGB = [
    [3.2410, -1.5374, -0.4986],
    [-0.9692, 1.8760, 0.0416],
    [0.0556, -0.2040, 1.0570]
];
// Color is sRGB with 0..255 range. Result is in D65 white point.
function sRGB_to_XYZ(rgb) {
    const rgblin = rgb.map(x => srgb_eotf(x / 255));
    return mat_vec_mul(SRGB_TO_XYZ, rgblin);
}
function XYZ_to_sRGB(xyz) {
    const rgblin = mat_vec_mul(XYZ_TO_SRGB, xyz);
    return rgblin.map(x => Math.round(255 * srgb_eotf_inv(x)));
}
const SRGB = {"to_xyz": sRGB_to_XYZ, "from_xyz": XYZ_to_sRGB};


// From Oklab article, then some numpy. Note these are transposed. I'm
// not sure I have the conventions right, but it is giving the right
// answer.
const OKLAB_M1 = [
    [ 0.8189,  0.3619, -0.1289],
    [ 0.033 ,  0.9293,  0.0361],
    [ 0.0482,  0.2644,  0.6339]
];
const OKLAB_M2 = [
    [ 0.2105,  0.7936, -0.0041],
    [ 1.978 , -2.4286,  0.4506],
    [ 0.0259,  0.7828, -0.8087]
];
const OKLAB_INV_M1 = [
    [ 1.227 , -0.5578,  0.2813],
    [-0.0406,  1.1123, -0.0717],
    [-0.0764, -0.4215,  1.5862]
];
const OKLAB_INV_M2 = [
    [ 1.0   ,  0.3963,  0.2158],
    [ 1.0   , -0.1056, -0.0639],
    [ 1.0   , -0.0895, -1.2915]
];
function Oklab_to_XYZ(lab) {
    const lms = mat_vec_mul(OKLAB_INV_M2, lab);
    const lmslin = lms.map(x => x * x * x);
    return mat_vec_mul(OKLAB_INV_M1, lmslin);
}
function XYZ_to_Oklab(xyz) {
    const lmslin = mat_vec_mul(OKLAB_M1, xyz);
    const lms = lmslin.map(Math.cbrt);
    return mat_vec_mul(OKLAB_M2, lms);
}
const OKLAB = {"to_xyz": Oklab_to_XYZ, "from_xyz": XYZ_to_Oklab};

function cielab_f(t) {
    const d = 6.0/29.0;
    if (t < d * d * d) {
        return t / (3 * d * d) + 4.0/29.0;
    } else {
        return Math.cbrt(t);
    }
}
function cielab_f_inv(t) {
    const d = 6.0/29.0;
    if (t < d) {
        return 3 * d * d * (t - 4.0/29.0);
    } else {
        return t * t * t;
    }
}
function XYZ_to_Lab(xyz) {
    // Just normalizing XYZ values to the white point is the "wrong von Kries"
    // transformation, which is faithful to the spec.
    const xyz_n = [xyz[0] / .9505, xyz[1], xyz[2] / 1.0888];
    const fxyz = xyz_n.map(cielab_f);
    const L = 116 * fxyz[1] - 16;
    const a = 500 * (fxyz[0] - fxyz[1]);
    const b = 200 * (fxyz[1] - fxyz[2]);
    return [L, a, b];
}
function Lab_to_XYZ(lab) {
    const l_ = (lab[0] + 16) / 116;
    const x = 0.9505 * cielab_f_inv(l_ + lab[1] / 500);
    const y = cielab_f_inv(l_);
    const z = 1.0888 * cielab_f_inv(l_ - lab[2] / 200);
    return [x, y, z];
}
const CIELAB = {"to_xyz": Lab_to_XYZ, "from_xyz": XYZ_to_Lab};

// https://professional.dolby.com/siteassets/pdfs/ictcp_dolbywhitepaper_v071.pdf
const ICTCP_XYZ_TO_LMS = [
    [ 0.3593,  0.6976, -0.0359],
    [-0.1921,  1.1005,  0.0754],
    [ 0.0071,  0.0748,  0.8433]
];
const ICTCP_LMS_TO_ITP = [
    [ 0.5   ,  0.5   ,  0.0   ],
    [ 1.6138, -3.3235,  1.7097],
    [ 4.3782, -4.2456, -0.1326]
];
const ICTCP_LMS_TO_XYZ = [
    [ 2.0703, -1.3265,  0.2067],
    [ 0.3647,  0.6806, -0.0453],
    [-0.0498, -0.0492,  1.1881]
];
const ICTCP_ITP_TO_LMS = [
    [ 1.0   ,  0.0086,  0.111 ],
    [ 1.0   , -0.0086, -0.111 ],
    [ 1.0   ,  0.56  , -0.3206]
];
const m1 = 2610/16384;
const m2 = 2523/4096 * 128;
const c2 = 2413/4096 * 32;
const c3 = 2392/4096 * 32;
const c1 = c3 - c2 + 1;
// This peak luminance value is from the Dolby whitepaper but seems too high.
const L_p = 10000;
// Note: 80 is what is specified by sRGB but seems too low; this value is chosen
// to be typical for actual non-HDR displays.
const L_display = 200;
function st_2084_eotf_inv(n) {
    const fd = n * L_display;
    const y = fd / L_p;
    const ym1 = Math.pow(y, m1);
    return Math.pow((c1 + c2 * ym1) / (1 + c3 * ym1), m2);
}
function st_2084_eotf(x) {
    const V_p = Math.pow(x, 1 / m2);
    const n = V_p - c1;
    // maybe max with 0 here?
    const L = Math.pow(n / (c2 - c3 * V_p), 1 / m1);
    return L * L_p / L_display;
}
function ICtCp_to_XYZ(lab) {
    const lms = mat_vec_mul(ICTCP_ITP_TO_LMS, lab);
    const lmslin = lms.map(st_2084_eotf);
    return mat_vec_mul(ICTCP_LMS_TO_XYZ, lmslin);
}
function XYZ_to_ICtCp(xyz) {
    const lmslin = mat_vec_mul(ICTCP_XYZ_TO_LMS, xyz);
    const lms = lmslin.map(st_2084_eotf_inv);
    return mat_vec_mul(ICTCP_LMS_TO_ITP, lms);
}
const ICTCP = {"to_xyz": ICtCp_to_XYZ, "from_xyz": XYZ_to_ICtCp};

///
const IPT_XYZ_TO_LMS = [
    [0.4002, 0.7075, -0.0807],
    [-0.2280, 1.1500, 0.0612],
    [0.0000, 0.0000, 0.9184]
];
const IPT_LMS_TO_IPT = [
    [0.4000, 0.4000, 0.2000],
    [4.4550, -4.8510, 0.3960],
    [0.8056, 0.3572, -1.1628],
];
const IPT_LMS_TO_XYZ = [
    [ 1.8502, -1.1383,  0.2384],
    [ 0.3668,  0.6439, -0.0107],
    [ 0.0   ,  0.0   ,  1.0889]
];
const IPT_IPT_TO_LMS = [
    [ 1.0   ,  0.0976,  0.2052],
    [ 1.0   , -0.1139,  0.1332],
    [ 1.0   ,  0.0326, -0.6769]
];
function IPT_to_XYZ(lab) {
    const lms = mat_vec_mul(IPT_IPT_TO_LMS, lab);
    const lmslin = lms.map(x => Math.pow(x, 1.0 / 0.43));
    return mat_vec_mul(IPT_LMS_TO_XYZ, lmslin);
}
function XYZ_to_IPT(xyz) {
    const lmslin = mat_vec_mul(IPT_XYZ_TO_LMS, xyz);
    const lms = lmslin.map(x => Math.pow(x, 0.43));
    return mat_vec_mul(IPT_LMS_TO_IPT, lms);
}
const IPT = {"to_xyz": IPT_to_XYZ, "from_xyz": XYZ_to_IPT};

function draw_gradient(id, c1, c2, cs, q) {
    const n_steps = Math.round(2.0 / (1 - Math.cbrt(q)));
    const a1 = cs["from_xyz"](sRGB_to_XYZ(c1));
    const a2 = cs["from_xyz"](sRGB_to_XYZ(c2));
    const element = document.getElementById(id);
    const w = element.width;
    const h = element.height;
    const ctx = element.getContext("2d");
    const img = ctx.createImageData(w, h);
    for (let x = 0; x < w; x++) {
        let t = x / (w - 1);
        if (q < 1) {
            t = Math.min(Math.floor(t * (n_steps + 1)) / n_steps, 1.0);
        }
        const a = lerp(a1, a2, t);
        const c = XYZ_to_sRGB(cs["to_xyz"](a));
        img.data[x * 4 + 0] = c[0];
        img.data[x * 4 + 1] = c[1];
        img.data[x * 4 + 2] = c[2];
        img.data[x * 4 + 3] = 255;
    }
    for (let y = 1; y < h; y++) {
        img.data.copyWithin(y * w * 4, 0, w * 4);
    }
    ctx.putImageData(img, 0, 0);
}

// UI
function getrgb(n) {
    return ['r', 'g', 'b'].map(c => {
        const v = document.getElementById(c + n).valueAsNumber;
        document.getElementById(c + 'o' + n).innerText = `${v}`;
        return v;
    });
}
function update(e) {
    rgb1 = getrgb(1);
    rgb2 = getrgb(2);
    q = document.getElementById('quant').valueAsNumber;
    draw_gradient("c0", rgb1, rgb2, SRGB, q);
    draw_gradient("c1", rgb1, rgb2, CIELAB, q);
    draw_gradient("c2", rgb1, rgb2, IPT, q);
    draw_gradient("c3", rgb1, rgb2, OKLAB, q);
    draw_gradient("c4", rgb1, rgb2, ICTCP, q);
}
function setrgb(rgb1, rgb2) {
    for (let i = 0; i < 3; i++) {
        const c = ['r', 'g', 'b'][i];
        document.getElementById(c + 1).valueAsNumber = rgb1[i];
        document.getElementById(c + 2).valueAsNumber = rgb2[i];
    }
    update();
}
function randomize(e) {
    const rgb1 = [0, 1, 2].map(_ => Math.round(255 * Math.random()));
    const rgb2 = [0, 1, 2].map(_ => Math.round(255 * Math.random()));
    setrgb(rgb1, rgb2);
}
function install_ui() {
    for (var c of ['r', 'g', 'b']) {
        document.getElementById(c + 1).addEventListener('input', update);
        document.getElementById(c + 2).addEventListener('input', update);
    }
    document.getElementById('quant').addEventListener('input', update);
    document.getElementById('randomize').addEventListener('click', randomize);
    const colors = [
        [[0, 0, 255], [255, 255, 255]],
        [[0, 0, 0], [255, 255, 255]],
        [[0, 0, 17], [255, 255, 255]],
        [[0, 0, 255], [255, 255, 0]],
        [[255, 0, 0], [0, 0, 255]],
        [[255, 0, 0], [0, 255, 0]]
    ];
    for (var i = 0; i < colors.length; i++) {
        const c = colors[i];
        document.getElementById('q' + i).addEventListener('click', e => {
            setrgb(c[0], c[1]);
        });
    }
}
install_ui();
update();
</script>
