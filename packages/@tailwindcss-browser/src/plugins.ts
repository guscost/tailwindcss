import plugin from "tailwindcss/plugin"

function filterDefault(values: Record<string, any>) {
  return Object.fromEntries(
    Object.entries(values).filter(([key]) => key !== "DEFAULT"),
  )
}

export const tailwindcssAnimate = plugin(
  ({ addUtilities, matchUtilities, theme }) => {
    addUtilities({
      "@keyframes enter": theme("keyframes.enter"),
      "@keyframes exit": theme("keyframes.exit"),
      ".animate-in": {
        animationName: "enter",
        animationDuration: theme("animationDuration.DEFAULT"),
        "--tw-enter-opacity": "initial",
        "--tw-enter-scale": "initial",
        "--tw-enter-rotate": "initial",
        "--tw-enter-translate-x": "initial",
        "--tw-enter-translate-y": "initial",
      },
      ".animate-out": {
        animationName: "exit",
        animationDuration: theme("animationDuration.DEFAULT"),
        "--tw-exit-opacity": "initial",
        "--tw-exit-scale": "initial",
        "--tw-exit-rotate": "initial",
        "--tw-exit-translate-x": "initial",
        "--tw-exit-translate-y": "initial",
      },
    })

    matchUtilities(
      {
        "fade-in": (value) => ({ "--tw-enter-opacity": value }),
        "fade-out": (value) => ({ "--tw-exit-opacity": value }),
      },
      { values: theme("animationOpacity") },
    )

    matchUtilities(
      {
        "zoom-in": (value) => ({ "--tw-enter-scale": value }),
        "zoom-out": (value) => ({ "--tw-exit-scale": value }),
      },
      { values: theme("animationScale") },
    )

    matchUtilities(
      {
        "spin-in": (value) => ({ "--tw-enter-rotate": value }),
        "spin-out": (value) => ({ "--tw-exit-rotate": value }),
      },
      { values: theme("animationRotate") },
    )

    matchUtilities(
      {
        "slide-in-from-top": (value) => ({
          "--tw-enter-translate-y": `-${value}`,
        }),
        "slide-in-from-bottom": (value) => ({
          "--tw-enter-translate-y": value,
        }),
        "slide-in-from-left": (value) => ({
          "--tw-enter-translate-x": `-${value}`,
        }),
        "slide-in-from-right": (value) => ({
          "--tw-enter-translate-x": value,
        }),
        "slide-out-to-top": (value) => ({
          "--tw-exit-translate-y": `-${value}`,
        }),
        "slide-out-to-bottom": (value) => ({
          "--tw-exit-translate-y": value,
        }),
        "slide-out-to-left": (value) => ({
          "--tw-exit-translate-x": `-${value}`,
        }),
        "slide-out-to-right": (value) => ({
          "--tw-exit-translate-x": value,
        }),
      },
      { values: theme("animationTranslate") },
    )

    matchUtilities(
      { duration: (value) => ({ animationDuration: value }) },
      { values: filterDefault(theme("animationDuration")) },
    )

    matchUtilities(
      { delay: (value) => ({ animationDelay: value }) },
      { values: theme("animationDelay") },
    )

    matchUtilities(
      { ease: (value) => ({ animationTimingFunction: value }) },
      { values: filterDefault(theme("animationTimingFunction")) },
    )

    addUtilities({
      ".running": { animationPlayState: "running" },
      ".paused": { animationPlayState: "paused" },
    })

    matchUtilities(
      { "fill-mode": (value) => ({ animationFillMode: value }) },
      { values: theme("animationFillMode") },
    )

    matchUtilities(
      { direction: (value) => ({ animationDirection: value }) },
      { values: theme("animationDirection") },
    )

    matchUtilities(
      { repeat: (value) => ({ animationIterationCount: value }) },
      { values: theme("animationRepeat") },
    )
  },
  {
    theme: {
      extend: {
        animationDelay: ({ theme }) => ({
          ...theme("transitionDelay"),
        }),
        animationDuration: ({ theme }) => ({
          0: "0ms",
          ...theme("transitionDuration"),
        }),
        animationTimingFunction: ({ theme }) => ({
          ...theme("transitionTimingFunction"),
        }),
        animationFillMode: {
          none: "none",
          forwards: "forwards",
          backwards: "backwards",
          both: "both",
        },
        animationDirection: {
          normal: "normal",
          reverse: "reverse",
          alternate: "alternate",
          "alternate-reverse": "alternate-reverse",
        },
        animationOpacity: ({ theme }) => ({
          DEFAULT: 0,
          ...theme("opacity"),
        }),
        animationTranslate: ({ theme }) => ({
          DEFAULT: "100%",
          ...theme("translate"),
        }),
        animationScale: ({ theme }) => ({
          DEFAULT: 0,
          ...theme("scale"),
        }),
        animationRotate: ({ theme }) => ({
          DEFAULT: "30deg",
          ...theme("rotate"),
        }),
        animationRepeat: {
          0: "0",
          1: "1",
          infinite: "infinite",
        },
        keyframes: {
          enter: {
            from: {
              opacity: "var(--tw-enter-opacity, 1)",
              transform:
                "translate3d(var(--tw-enter-translate-x, 0), var(--tw-enter-translate-y, 0), 0) scale3d(var(--tw-enter-scale, 1), var(--tw-enter-scale, 1), var(--tw-enter-scale, 1)) rotate(var(--tw-enter-rotate, 0))",
            },
          },
          exit: {
            to: {
              opacity: "var(--tw-exit-opacity, 1)",
              transform:
                "translate3d(var(--tw-exit-translate-x, 0), var(--tw-exit-translate-y, 0), 0) scale3d(var(--tw-exit-scale, 1), var(--tw-exit-scale, 1), var(--tw-exit-scale, 1)) rotate(var(--tw-exit-rotate, 0))",
            },
          },
        },
      },
    },
  },
)

const baseStyles = {
  position: 'relative',
  paddingBottom: `calc(var(--tw-aspect-h) / var(--tw-aspect-w) * 100%)`,
}

const childStyles = {
  position: 'absolute',
  height: '100%',
  width: '100%',
  top: '0',
  right: '0',
  bottom: '0',
  left: '0',
}

const noneComponent = {
  '.aspect-none': {
    position: 'static',
    paddingBottom: '0',
  },
  '.aspect-none > *': {
    position: 'static',
    height: 'auto',
    width: 'auto',
    top: 'auto',
    right: 'auto',
    bottom: 'auto',
    left: 'auto',
  },
}

export const tailwindcssAspectRatio = plugin(
  function ({ addComponents, matchComponents, theme, variants, e }) {
    const values = theme('aspectRatio')

    if (matchComponents) {
      matchComponents(
        {
          'aspect-w': (value) => [
            {
              ...baseStyles,
              '--tw-aspect-w': value,
            },
            {
              '> *': childStyles,
            },
          ],
          'aspect-h': (value) => ({ '--tw-aspect-h': value }),
        },
        { values }
      )

      addComponents(noneComponent)

      return
    }

    const baseSelectors = Object.entries(values)
      .map(([key, value]) => {
        return `.${e(`aspect-w-${key}`)}`
      })
      .join(',\n')

    const childSelectors = Object.entries(values)
      .map(([key, value]) => {
        return `.${e(`aspect-w-${key}`)} > *`
      })
      .join(',\n')

    addComponents(
      [
        {
          [baseSelectors]: baseStyles,
          [childSelectors]: childStyles,
        },
        noneComponent,
        Object.entries(values).map(([key, value]) => {
          return {
            [`.${e(`aspect-w-${key}`)}`]: {
              '--tw-aspect-w': value,
            },
          }
        }),
        Object.entries(values).map(([key, value]) => {
          return {
            [`.${e(`aspect-h-${key}`)}`]: {
              '--tw-aspect-h': value,
            },
          }
        }),
      ],
      variants('aspectRatio')
    )
  },
  {
    theme: {
      aspectRatio: {
        1: '1',
        2: '2',
        3: '3',
        4: '4',
        5: '5',
        6: '6',
        7: '7',
        8: '8',
        9: '9',
        10: '10',
        11: '11',
        12: '12',
        13: '13',
        14: '14',
        15: '15',
        16: '16',
      },
    },
    variants: {
      aspectRatio: ['responsive'],
    },
  }
)

export const tailwindcssContainerQueries = plugin(
  function containerQueries({ matchUtilities, matchVariant, theme }) {
    let values: Record<string, string> = theme('containers') ?? {}

    function parseValue(value: string) {
      let numericValue = value.match(/^(\d+\.\d+|\d+|\.\d+)\D+/)?.[1] ?? null
      if (numericValue === null) return null

      return parseFloat(value)
    }

    matchUtilities(
      {
        '@container': (value, { modifier }) => {
          return {
            'container-type': value,
            'container-name': modifier,
          }
        },
      },
      {
        values: {
          DEFAULT: 'inline-size',
          normal: 'normal',
        },
        modifiers: 'any',
      }
    )

    matchVariant(
      '@',
      (value = '', { modifier }) => {
        let parsed = parseValue(value)

        return parsed !== null ? `@container ${modifier ?? ''} (min-width: ${value})` : []
      },
      {
        values,
        sort(aVariant, zVariant) {
          let a = parseFloat(aVariant.value)
          let z = parseFloat(zVariant.value)

          if (a === null || z === null) return 0

          // Sort values themselves regardless of unit
          if (a - z !== 0) return a - z

          let aLabel = aVariant.modifier ?? ''
          let zLabel = zVariant.modifier ?? ''

          // Explicitly move empty labels to the end
          if (aLabel === '' && zLabel !== '') {
            return 1
          } else if (aLabel !== '' && zLabel === '') {
            return -1
          }

          // Sort labels alphabetically in the English locale
          // We are intentionally overriding the locale because we do not want the sort to
          // be affected by the machine's locale (be it a developer or CI environment)
          return aLabel.localeCompare(zLabel, 'en', { numeric: true })
        },
      }
    )
  },
  {
    theme: {
      containers: {
        xs: '20rem',
        sm: '24rem',
        md: '28rem',
        lg: '32rem',
        xl: '36rem',
        '2xl': '42rem',
        '3xl': '48rem',
        '4xl': '56rem',
        '5xl': '64rem',
        '6xl': '72rem',
        '7xl': '80rem',
      },
    },
  }
)
