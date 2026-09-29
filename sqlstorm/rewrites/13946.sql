WITH UserPerformance AS (
    SELECT 
        u.Id AS UserId,
        u.Reputation,
        COUNT(p.Id) AS TotalPosts,
        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
    FROM Users u
    LEFT JOIN Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN Votes v ON p.Id = v.PostId
    GROUP BY u.Id, u.Reputation
),

AveragePostAge AS (
    SELECT 
        u.Id AS UserId,
        SUM(epoch_us(cast('2024-10-01 12:34:56' as timestamp)) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.CreationDate) / 1e6 / 86400 AS AveragePostAgeInDays
    FROM Users u
    JOIN Posts p ON u.Id = p.OwnerUserId
    GROUP BY u.Id
)

SELECT 
    up.UserId,
    up.Reputation,
    up.TotalPosts,
    up.TotalQuestions,
    up.TotalAnswers,
    up.TotalUpVotes,
    up.TotalDownVotes,
    apa.AveragePostAgeInDays
FROM UserPerformance up
JOIN AveragePostAge apa ON up.UserId = apa.UserId
ORDER BY up.Reputation DESC;