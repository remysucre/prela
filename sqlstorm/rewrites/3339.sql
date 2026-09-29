
WITH UserPostStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(p.Id) AS TotalPosts,
        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
        AVG(COALESCE(v.UpVotes, 0) - COALESCE(v.DownVotes, 0)) AS AvgScore
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN 
        (SELECT 
             PostId,
             SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
             SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
         FROM 
             Votes
         GROUP BY 
             PostId) v ON p.Id = v.PostId
    GROUP BY 
        u.Id, u.DisplayName
),
RecentActiveUsers AS (
    SELECT 
        p.OwnerUserId AS UserId,
        COUNT(*) AS RecentActivityCount
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'
    GROUP BY 
        p.OwnerUserId
),
PopularQuestions AS (
    SELECT 
        p.Id,
        p.Title,
        p.CreationDate,
        COALESCE(v.UpVotes, 0) - COALESCE(v.DownVotes, 0) AS Score
    FROM 
        Posts p
    LEFT JOIN 
        (SELECT 
             PostId,
             SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
             SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
         FROM 
             Votes
         GROUP BY 
             PostId) v ON p.Id = v.PostId
    WHERE 
        p.PostTypeId = 1
    ORDER BY Score DESC, p.Id LIMIT 10
)
SELECT 
    ups.DisplayName,
    ups.TotalPosts,
    ups.TotalQuestions,
    ups.TotalAnswers,
    ups.AvgScore,
    rau.RecentActivityCount,
    pq.Title AS PopularQuestionTitle,
    pq.Score AS PopularQuestionScore
FROM 
    UserPostStats ups
LEFT JOIN 
    RecentActiveUsers rau ON ups.UserId = rau.UserId
LEFT JOIN 
    PopularQuestions pq ON pq.Id IS NOT NULL
ORDER BY ups.AvgScore DESC, ups.TotalPosts DESC, ups.UserId, pq.Score DESC, pq.Id LIMIT 5;
