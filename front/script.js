import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import Lenis from "lenis";

gsap.registerPlugin(ScrollTrigger);

document.addEventListener("DOMContentLoaded", () => {
  const lenis = new Lenis({ lerp: 0.08, smoothWheel: true });
  lenis.on("scroll", ScrollTrigger.update);
  gsap.ticker.add((time) => {
    lenis.raf(time * 1000);
  });
  gsap.ticker.lagSmoothing(0);

  const path = document.getElementById("stroke-path");
  if (path) {
    const pathLength = path.getTotalLength();
    path.style.strokeDasharray = pathLength;
    path.style.strokeDashoffset = pathLength;

    gsap.to(path, {
      strokeDashoffset: 0,
      ease: "none",
      scrollTrigger: {
        trigger: ".spotlight",
        start: "top top",
        end: "bottom bottom",
        scrub: 2,
      },
    });
  }

  document.querySelectorAll(".spotlight .card").forEach((card) => {
    const tl = gsap.timeline({
      scrollTrigger: {
        trigger: card,
        start: "top bottom",
        end: "bottom top",
        scrub: 2,
      },
    });
    tl.fromTo(card, { opacity: 0 }, { opacity: 1, duration: 0.25, ease: "none" }, 0);
    tl.to(card, { opacity: 0, duration: 0.25, ease: "none" }, 0.75);
  });

  const html = document.documentElement;
  const themeToggle = document.querySelector(".theme-toggle");
  const saved = localStorage.getItem("ttc-theme");
  if (saved === "light") html.setAttribute("data-theme", "light");

  themeToggle?.addEventListener("click", () => {
    const isLight = html.getAttribute("data-theme") === "light";
    html.setAttribute("data-theme", isLight ? "" : "light");
    localStorage.setItem("ttc-theme", isLight ? "dark" : "light");
  });
});
