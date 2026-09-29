
WITH UserPostStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(p.Id) AS PostCount,
        COALESCE(SUM(vote.Score), 0) AS TotalVotes,
        COALESCE(SUM(c.CommentCount), 0) AS TotalComments,
        SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.CreationDate) / 1e6 AS AvgActivityDuration
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN 
        (SELECT PostId, COUNT(*) AS CommentCount
         FROM Comments
         GROUP BY PostId) c ON p.Id = c.PostId
    LEFT JOIN 
        (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS Score
         FROM Votes
         GROUP BY PostId) vote ON p.Id = vote.PostId
    GROUP BY 
        u.Id, u.DisplayName
)

SELECT 
    ups.DisplayName,
    ups.PostCount,
    ups.TotalVotes,
    ups.TotalComments,
    ups.AvgActivityDuration
FROM 
    UserPostStats ups
JOIN 
    Users u ON ups.UserId = u.Id
ORDER BY 
    ups.PostCount DESC, ups.TotalVotes DESC
FETCH FIRST 10 ROWS ONLY;
