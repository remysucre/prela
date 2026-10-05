WITH UserStats AS (
    SELECT
        u.Id AS UserId,
        u.DisplayName,
        u.Reputation,
        u.CreationDate,
        COUNT(DISTINCT p.Id) AS TotalPosts,
        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
        SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
        AVG(vote_count) AS AvgVotes
    FROM Users u
    LEFT JOIN Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN (
        SELECT 
            PostId,
            COUNT(*) AS vote_count
        FROM Votes
        GROUP BY PostId
    ) v ON p.Id = v.PostId
    GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate
),

TopTags AS (
    SELECT
        t.TagName,
        COUNT(p.Id) AS PostCount
    FROM Tags t
    JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
    GROUP BY t.TagName
    ORDER BY PostCount DESC
    LIMIT 5
)

SELECT
    us.UserId,
    us.DisplayName,
    us.Reputation,
    us.TotalPosts,
    us.Questions,
    us.Answers,
    us.AcceptedAnswers,
    us.AvgVotes,
    STRING_AGG(tt.TagName, ', ' ORDER BY tt.PostCount DESC) AS PopularTags
FROM UserStats us
LEFT JOIN TopTags tt ON us.Questions > 0
GROUP BY us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.Questions, us.Answers, us.AcceptedAnswers, us.AvgVotes
HAVING us.Reputation > (SELECT AVG(Reputation) FROM Users WHERE Reputation IS NOT NULL)
ORDER BY us.Reputation DESC
LIMIT 10;
