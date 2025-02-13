class BackgroundVideoManager {
    constructor(cycleInterval = 15000) { // Default 15 second interval
        this.videos = Array.from(document.querySelectorAll('.bg-video'));
        this.currentIndex = 0;
        this.cycleInterval = cycleInterval;
        
        // Start cycling if there are multiple videos
        if (this.videos.length > 1) {
            this.startCycle();
        }
    }

    switchToVideo(index) {
        // Remove active class from all videos
        this.videos.forEach(video => {
            video.classList.remove('active');
            video.pause(); // Pause inactive videos to save resources
        });

        // Add active class to new video
        const nextVideo = this.videos[index];
        nextVideo.classList.add('active');
        nextVideo.play();
    }

    startCycle() {
        setInterval(() => {
            this.currentIndex = (this.currentIndex + 1) % this.videos.length;
            this.switchToVideo(this.currentIndex);
        }, this.cycleInterval);
    }
}

// Initialize when the DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    new BackgroundVideoManager();
}); 