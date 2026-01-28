// Copy button functionality
document.querySelectorAll('.copy-btn').forEach((btn) => {
  btn.addEventListener('click', async () => {
    const target = btn.getAttribute('data-copy');
    const codeEl = document.getElementById(`${target}-code`);
    
    if (codeEl) {
      // Get text content, stripping HTML
      const text = codeEl.textContent || '';
      
      try {
        await navigator.clipboard.writeText(text);
        btn.classList.add('copied');
        
        // Show checkmark briefly
        const originalHTML = btn.innerHTML;
        btn.innerHTML = `
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M20 6L9 17l-5-5"/>
          </svg>
        `;
        
        setTimeout(() => {
          btn.classList.remove('copied');
          btn.innerHTML = originalHTML;
        }, 2000);
      } catch (err) {
        console.error('Failed to copy:', err);
      }
    }
  });
});

// Smooth scroll for anchor links
document.querySelectorAll('a[href^="#"]').forEach((anchor) => {
  anchor.addEventListener('click', (e) => {
    e.preventDefault();
    const href = anchor.getAttribute('href');
    if (href) {
      const target = document.querySelector(href);
      if (target) {
        target.scrollIntoView({
          behavior: 'smooth',
          block: 'start',
        });
      }
    }
  });
});

// Add intersection observer for fade-in animations on scroll
const observerOptions = {
  threshold: 0.1,
  rootMargin: '0px 0px -50px 0px',
};

const observer = new IntersectionObserver((entries) => {
  entries.forEach((entry) => {
    if (entry.isIntersecting) {
      entry.target.classList.add('visible');
      observer.unobserve(entry.target);
    }
  });
}, observerOptions);

// Observe sections for scroll animations
document.querySelectorAll('.problem-card, .solution-card, .feature-card, .code-block').forEach((el) => {
  el.classList.add('fade-in-target');
  observer.observe(el);
});

// Add CSS for scroll animations
const style = document.createElement('style');
style.textContent = `
  .fade-in-target {
    opacity: 0;
    transform: translateY(20px);
    transition: opacity 0.5s ease, transform 0.5s ease;
  }
  .fade-in-target.visible {
    opacity: 1;
    transform: translateY(0);
  }
`;
document.head.appendChild(style);
